// SPDX-License-Identifier: GPL-3.0-only
//! Actual xLog retention/HTTP and xNexus audio client, with synthetic PCM only.
use super::{
    companion_bridge::Bridge,
    companion_rx_buffer::{Binding, Buffer, Route},
};
use std::sync::{Arc, Mutex};
use xnexus_xlog_client::{audio::AudioState, BridgeClient, Error};

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn route() -> (Binding, Route) {
    let b = Binding {
        device_id: "synthetic-dax".into(),
        radio_id: "radio1".into(),
        receiver_id: "synthetic-slice-a".into(),
        dax_channel: 2,
        sample_rate: 48000,
        source_channels: 2,
        selected_channel: 1,
        operator_confirmed: true,
    };
    let r = Route {
        radio_id: b.radio_id.clone(),
        receiver_id: b.receiver_id.clone(),
        connection_generation: 1,
        selection_generation: 1,
        dax_channel: 2,
        frequency_hz: 14074000,
        mode: "DIGU".into(),
    };
    (b, r)
}

#[test]
fn actual_owner_audio_pages_selected_samples_and_resets_after_restart() {
    let (binding, route) = route();
    let mut buffer = Buffer::default();
    buffer.start(&binding, &route, 1).unwrap();
    let buffer = Arc::new(Mutex::new(buffer));
    let owner = buffer.clone();
    let bridge = Bridge::start(
        "audio-workspace".into(),
        Arc::new(move |request| {
            // The real hello route also asks the provider to confirm availability.
            let Some(request) = request else {
                return Ok(serde_json::json!({}));
            };
            let after = request["audio_after"]
                .as_u64()
                .ok_or("Expected bounded audio cursor")?;
            Ok(owner.lock().unwrap().response(after, now()))
        }),
    )
    .unwrap();
    let mut client = BridgeClient::new(&bridge.endpoint, &bridge.token).unwrap();
    client.connect().unwrap();
    let mut audio = client.audio_handle().unwrap();
    // Produce only after HTTP/client initialization: a frozen pre-launch buffer
    // can correctly expire while concurrent build/tests load the machine.
    let base = (now() - 400) * 10000;
    {
        let mut buffer = buffer.lock().unwrap();
        for n in 0..40 {
            buffer
                .push(
                    n * 480,
                    10_000_000 + n * 100000,
                    base + n * 100000,
                    100,
                    &[n as i16; 480],
                )
                .unwrap();
        }
    }
    let first = audio.poll().unwrap();
    assert!(first.reset && first.is_fresh());
    assert_eq!(first.chunks.len(), 32);
    assert_eq!(first.chunks[31].samples, vec![31; 480]);
    assert_eq!(first.descriptor.as_ref().unwrap().selected_channel, 1);
    assert_eq!(first.descriptor.as_ref().unwrap().dax_channel, 2);
    let second = audio.poll().unwrap();
    assert!(!second.reset);
    assert_eq!(second.chunks.len(), 8);
    assert_eq!(second.chunks[0].sequence, 33);
    assert_eq!(second.chunks[7].samples, vec![39; 480]);
    assert!(audio.poll().unwrap().chunks.is_empty());

    buffer.lock().unwrap().stop("Synthetic QSY");
    let stopped = audio.poll().unwrap();
    assert_eq!(stopped.state, AudioState::Stopped);
    assert!(stopped.reset && stopped.descriptor.is_none() && stopped.chunks.is_empty());
    let mut changed = route.clone();
    changed.frequency_hz = 7074000;
    {
        let mut b = buffer.lock().unwrap();
        b.start(&binding, &changed, 2).unwrap();
        b.push(0, 20_000_000, now() * 10000, 100, &[-123; 480])
            .unwrap();
    }
    let restarted = audio.poll().unwrap();
    assert!(restarted.reset);
    assert_eq!(restarted.chunks[0].sequence, 41);
    assert_eq!(restarted.descriptor.as_ref().unwrap().frequency_hz, 7074000);
    assert_ne!(
        first.descriptor.as_ref().unwrap().epoch,
        restarted.descriptor.as_ref().unwrap().epoch
    );
    bridge.revoke();
    assert!(matches!(
        audio.poll(),
        Err(Error::Http(401) | Error::Transport)
    ));
    assert!(!restarted.is_fresh());
    assert!(matches!(audio.poll(), Err(Error::Disconnected)));
}
