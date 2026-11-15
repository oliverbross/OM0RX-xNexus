//! DSP/lifecycle unit fixtures. Generated samples and synthetic HTTP authority;
//! these are not device, GUI, or physical RF acceptance.
use super::super::{
    tests::{authority_batches, live_authority, state},
    Mode, Observation, Worker, ACTIVE,
};
use super::*;

fn context(batch: &AudioBatch) -> NeedsContext {
    NeedsContext {
        worker_instance: "fixture".into(),
        worker_epoch: 1,
        server_instance: "worker-fixture".into(),
        workspace_id: "worker-workspace".into(),
        route: batch.descriptor.clone().unwrap(),
        decoder_mode: Mode::FT8,
    }
}
fn span(samples: &[f32], first: u64) -> ValidatedSamples<'_> {
    ValidatedSamples {
        samples,
        origin_device_frame: 0,
        origin_unix_us: 1_700_000_000_000_006,
        first_output_index: first,
        source_rate: 48000,
        host_mapping_uncertainty_us: 20,
        calibration_uncertainty_us: 10,
    }
}
fn feed(producer: &mut Producer, batch: Arc<AudioBatch>, samples: &[f32], first: u64) {
    producer.input(batch.clone(), (first + samples.len() as u64) * 4 + 64);
    producer.push(context(&batch), span(samples, first));
}
fn tone(count: usize) -> Vec<f32> {
    (0..count)
        .map(|n| 0.5 * (2.0 * std::f32::consts::PI * 1500.0 * n as f32 / 12000.0).sin())
        .collect()
}

#[test]
fn complete_real_window_matches_original_nexus_fft_and_exact_host_position() {
    let (_bridge, _audio, batch) = live_authority();
    let mailbox = Arc::new(Mailbox::default());
    let (wake, _rx) = mpsc::sync_channel(1);
    let mut producer = Producer::new(mailbox.clone(), wake);
    let samples = tone(WINDOW);
    feed(&mut producer, batch.clone(), &samples[..WINDOW - 1], 0);
    assert!(mailbox.pending.lock().unwrap().is_none());
    feed(
        &mut producer,
        batch,
        &samples[WINDOW - 1..],
        (WINDOW - 1) as u64,
    );
    let mut window = mailbox.pending.lock().unwrap().take().unwrap();
    assert_eq!(window.samples, samples);
    window.frame.spectrum.row =
        tempo_core::spectrum::power_spectrum(&samples, 12000.0, 0.0, 5000.0, BINS);
    let peak = window
        .frame
        .spectrum
        .row
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .unwrap();
    assert!((191..=193).contains(&peak.0));
    assert!((*peak.1 - ((120.0 - 6.0206) / 120.0)).abs() < 0.01);
    assert_eq!(window.frame.first_output_index, 0);
    assert_eq!(window.frame.fir_center_source_frames_twice, 63);
    assert_eq!(window.frame.window_start_unix_us, 1_700_000_000_000_662);
    assert_eq!(window.frame.window_end_unix_us, 1_700_000_000_171_328);
    assert!(window.frame.utc_sync_accuracy_us.is_none());
    assert!(window.frame.remote_rf_latency_us.is_none());
    let published = Published {
        frame: window.frame,
        authorities: window.authorities,
        accepted: window.accepted,
    };
    let first = published.read(None).unwrap();
    assert!(first.remaining_valid_ms <= 500);
    println!(
        "XNEXUS_SPECTRUM_FIXTURE {}",
        serde_json::to_string(&first).unwrap()
    );
    let mut cursor = Cursor {
        worker_instance: first.context.worker_instance.clone(),
        worker_epoch: first.context.worker_epoch,
        sequence: first.sequence,
    };
    assert!(published.read(Some(&cursor)).is_none());
    cursor.worker_epoch += 1;
    assert!(
        published.read(Some(&cursor)).is_some(),
        "old route cursor cannot suppress new context"
    );
    cursor.worker_epoch = first.context.worker_epoch;
    cursor.worker_instance = "previous-worker".into();
    assert!(
        published.read(Some(&cursor)).is_some(),
        "old worker cursor cannot suppress a restarted feed"
    );
    assert!(published.read(None).unwrap().remaining_valid_ms <= first.remaining_valid_ms);
}

#[test]
fn latest_only_queue_reset_and_revocation_never_reuse_old_samples() {
    let (mut bridge, _audio, batch) = live_authority();
    let mailbox = Arc::new(Mailbox::default());
    let (wake, _rx) = mpsc::sync_channel(1);
    let mut producer = Producer::new(mailbox.clone(), wake);
    feed(&mut producer, batch.clone(), &tone(WINDOW), 0);
    let first_sequence = mailbox
        .pending
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .frame
        .sequence;
    feed(&mut producer, batch.clone(), &tone(WINDOW), WINDOW as u64);
    assert!(
        mailbox
            .pending
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .frame
            .sequence
            > first_sequence
    );
    assert_eq!(producer.samples.len(), WINDOW);
    producer.reset();
    assert!(mailbox.pending.lock().unwrap().is_none());
    feed(&mut producer, batch.clone(), &[0.2; WINDOW - 1], 0);
    assert!(mailbox.pending.lock().unwrap().is_none());
    bridge.disconnect();
    feed(&mut producer, batch, &[0.2], (WINDOW - 1) as u64);
    assert!(mailbox.pending.lock().unwrap().is_none());
    assert!(producer.samples.is_empty());
}

#[test]
fn expired_old_window_input_is_not_renewed_by_a_newer_batch() {
    let (mut old_bridge, _old_audio, old) = live_authority();
    let (_new_bridge, _new_audio, fresh) = live_authority();
    let mailbox = Arc::new(Mailbox::default());
    let (wake, _rx) = mpsc::sync_channel(1);
    let mut producer = Producer::new(mailbox.clone(), wake);
    feed(&mut producer, old, &[0.1; WINDOW - 1], 0);
    old_bridge.disconnect();
    feed(&mut producer, fresh, &[0.1], (WINDOW - 1) as u64);
    assert!(mailbox.pending.lock().unwrap().is_none());
}

#[test]
fn late_computed_window_is_rejected_after_every_context_boundary() {
    for boundary in [
        "epoch",
        "route",
        "mode",
        "worker",
        "workspace",
        "server",
        "age",
    ] {
        let (_bridge, _audio, batch) = live_authority();
        let mailbox = Arc::new(Mailbox::default());
        let (wake, _rx) = mpsc::sync_channel(1);
        let mut producer = Producer::new(mailbox.clone(), wake);
        feed(&mut producer, batch.clone(), &tone(WINDOW), 0);
        let mut job = mailbox.pending.lock().unwrap().take().unwrap();
        job.frame.spectrum.row =
            tempo_core::spectrum::power_spectrum(&job.samples, 12000.0, 0.0, 5000.0, BINS);
        let mut current = state(batch);
        match boundary {
            "epoch" => current.epoch += 1,
            "route" => current.view.route.as_mut().unwrap().frequency_hz += 1,
            "mode" => current.view.mode = Some(Mode::FT4),
            "worker" => current.spectrum_identity.0 = "new-worker".into(),
            "workspace" => current.spectrum_identity.2 = "new-workspace".into(),
            "server" => current.spectrum_identity.1 = "new-server".into(),
            "age" => job.accepted = Instant::now() - MAX_PICTURE_AGE,
            _ => unreachable!(),
        }
        let current = Mutex::new(current);
        assert!(
            !publish(&current, &AtomicBool::new(false), job),
            "late {boundary} result"
        );
        assert!(current.lock().unwrap().spectrum.is_none());
    }
}

#[test]
fn stalled_fft_does_not_block_pcm_or_stop_and_cannot_publish_after_retirement() {
    let _serial = super::super::tests::ACTIVE_TEST_LOCK.lock().unwrap();
    assert!(!ACTIVE.swap(true, Ordering::AcqRel));
    let (_bridge, _audio, batch) = live_authority();
    let shared = Arc::new(Mutex::new(state(batch.clone())));
    let mailbox = shared.lock().unwrap().spectrum_mailbox.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    let lease = Arc::new(Lease);
    let (wake, rx) = mpsc::sync_channel(1);
    let (entered, entered_rx) = mpsc::sync_channel(1);
    let (release, release_rx) = mpsc::sync_channel(1);
    let thread = spawn_with(
        shared.clone(),
        cancel.clone(),
        lease.clone(),
        mailbox.clone(),
        rx,
        move |samples| {
            entered.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            tempo_core::spectrum::power_spectrum(samples, 12000.0, 0.0, 5000.0, BINS)
        },
    )
    .unwrap();
    drop(lease);
    let mut producer = Producer::new(mailbox, wake.clone());
    feed(&mut producer, batch.clone(), &tone(WINDOW), 0);
    entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    // Exact production mailbox remains writable while compute is blocked.
    feed(&mut producer, batch, &tone(WINDOW), WINDOW as u64);
    assert_eq!(producer.samples.len(), WINDOW);
    let worker = Worker {
        state: shared.clone(),
        cancel,
        needs_wake: None,
        needs_observation: Arc::new(Observation::default()),
        spectrum_wake: wake,
        spectrum_thread: Mutex::new(Some(thread)),
    };
    worker.stop();
    assert!(!worker.view().unwrap().enabled);
    assert!(worker.spectrum(None).unwrap().is_none());
    assert!(
        ACTIVE.load(Ordering::Acquire),
        "old FFT retains worker lease until actual exit"
    );
    release.send(()).unwrap();
    let thread = worker.spectrum_thread.lock().unwrap().take().unwrap();
    thread.join().unwrap();
    assert!(!ACTIVE.load(Ordering::Acquire));
    assert!(shared.lock().unwrap().spectrum.is_none());
}

#[test]
fn real_assembler_split_fir_output_feeds_original_fft_at_native_rates() {
    for rate in [12000, 44100, 192000] {
        let (_bridge, _audio, batches) = authority_batches(rate, &[1000]);
        let batch = batches[0].clone();
        let ctx = context(&batch);
        let rx_context = super::super::context(
            &ctx.route,
            &ctx.server_instance,
            &ctx.workspace_id,
            Mode::FT8,
        );
        let mailbox = Arc::new(Mailbox::default());
        let (wake, _rx) = mpsc::sync_channel(1);
        let mut producer = Producer::new(mailbox.clone(), wake);
        let mut assembler = xnexus_xlog_rx::RxAssembler::default();
        let samples: Vec<i16> = (0..rate / 2)
            .map(|n| {
                (16000.0
                    * (2.0 * std::f64::consts::PI * 1500.0 * f64::from(n) / f64::from(rate)).sin())
                    as i16
            })
            .collect();
        let mut observed = Vec::new();
        let mut first = 0usize;
        let mut sequence = 1;
        let sizes = [1, 31, 32, 63, 127, 4096];
        while first < samples.len() {
            let end = (first + sizes[(sequence as usize - 1) % sizes.len()]).min(samples.len());
            producer.input(batch.clone(), end as u64);
            assembler
                .push_observed(
                    &rx_context,
                    xnexus_xlog_rx::PcmChunk {
                        sequence,
                        first_frame: first as u64,
                        sample0_unix_us: 1_700_000_000_000_006
                            + first as u64 * 1_000_000 / u64::from(rate),
                        calibration_uncertainty_us: 10,
                        samples: &samples[first..end],
                    },
                    |span| {
                        assert_eq!(span.first_output_index, observed.len() as u64);
                        observed.extend_from_slice(span.samples);
                        producer.push(ctx.clone(), span);
                    },
                )
                .unwrap();
            first = end;
            sequence += 1;
        }
        let job = mailbox.pending.lock().unwrap().take().unwrap();
        let first = job.frame.first_output_index as usize;
        assert_eq!(job.samples, observed[first..first + WINDOW]);
        assert_eq!(job.frame.source_sample_rate_hz, rate);
        assert_eq!(
            job.frame.fir_center_source_frames_twice,
            if rate == 12000 { 0 } else { 63 }
        );
        let actual = tempo_core::spectrum::power_spectrum(&job.samples, 12000.0, 0.0, 5000.0, BINS);
        assert_eq!(actual.len(), BINS);
        let peak = actual
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap();
        assert!(
            (191..=193).contains(&peak.0),
            "rate {rate}, peak {}",
            peak.0
        );
        assert!(
            *peak.1 > 0.90 && *peak.1 < 0.97,
            "real amplitude/rate {rate}"
        );
    }
}

#[test]
fn same_session_new_lease_cannot_renew_expired_contributing_samples() {
    let (_bridge, _audio, batches) = authority_batches(48000, &[100, 1000]);
    let old = batches[0].clone();
    let fresh = batches[1].clone();
    let mailbox = Arc::new(Mailbox::default());
    let (wake, _rx) = mpsc::sync_channel(1);
    let mut producer = Producer::new(mailbox.clone(), wake);
    feed(&mut producer, old.clone(), &[0.1; WINDOW - 1], 0);
    let deadline = Instant::now() + Duration::from_millis(300);
    while old.is_fresh() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!old.is_fresh(), "ordinary original deadline must elapse");
    assert!(
        fresh.is_fresh(),
        "same live session/newer response still valid"
    );
    feed(&mut producer, fresh, &[0.1], (WINDOW - 1) as u64);
    assert!(mailbox.pending.lock().unwrap().is_none());
    assert!(producer.samples.is_empty());
}

fn test_worker(current: State) -> Worker {
    Worker {
        state: Arc::new(Mutex::new(current)),
        cancel: Arc::new(AtomicBool::new(false)),
        needs_wake: None,
        needs_observation: Arc::new(Observation::default()),
        spectrum_wake: mpsc::sync_channel(1).0,
        spectrum_thread: Mutex::new(None),
    }
}
fn computed(batch: Arc<AudioBatch>) -> Window {
    let mailbox = Arc::new(Mailbox::default());
    let (wake, _rx) = mpsc::sync_channel(1);
    let mut producer = Producer::new(mailbox.clone(), wake);
    feed(&mut producer, batch, &tone(WINDOW), 0);
    let mut job = mailbox.pending.lock().unwrap().take().unwrap();
    job.frame.spectrum.row =
        tempo_core::spectrum::power_spectrum(&job.samples, 12000.0, 0.0, 5000.0, BINS);
    job
}

#[test]
fn completed_fft_cannot_be_read_after_original_input_lease_expires() {
    let (_bridge, _audio, batches) = authority_batches(48000, &[100, 1000]);
    let old = batches[0].clone();
    let fresh = batches[1].clone();
    let worker = test_worker(state(fresh.clone()));
    assert!(publish(
        &worker.state,
        &worker.cancel,
        computed(old.clone())
    ));
    assert!(worker.spectrum(None).unwrap().is_some());
    let deadline = Instant::now() + Duration::from_millis(300);
    while old.is_fresh() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!old.is_fresh());
    assert!(fresh.is_fresh());
    assert!(
        worker.state.lock().unwrap().accepts(1),
        "fresh current RX does not renew old FFT"
    );
    assert!(worker.spectrum(None).unwrap().is_none());
}

#[test]
fn worker_read_fences_full_context_even_if_published_frame_survives_reset() {
    for boundary in ["epoch", "route", "mode", "worker", "workspace", "server"] {
        let (_bridge, _audio, batch) = live_authority();
        let worker = test_worker(state(batch.clone()));
        assert!(publish(&worker.state, &worker.cancel, computed(batch)));
        assert!(worker.spectrum(None).unwrap().is_some());
        {
            let mut current = worker.state.lock().unwrap();
            match boundary {
                "epoch" => current.epoch += 1,
                "route" => current.view.route.as_mut().unwrap().mapping_generation += 1,
                "mode" => current.view.mode = Some(Mode::FT4),
                "worker" => current.spectrum_identity.0 = "replacement".into(),
                "workspace" => current.spectrum_identity.2 = "replacement".into(),
                "server" => current.spectrum_identity.1 = "replacement".into(),
                _ => unreachable!(),
            }
        }
        assert!(
            worker.spectrum(None).unwrap().is_none(),
            "read fence: {boundary}"
        );
    }
}

#[test]
fn receive_view_exports_current_context_before_decode_and_clears_it_on_retirement() {
    let (mut bridge, _audio, batch) = live_authority();
    let worker = test_worker(state(batch));
    let view = worker.view().unwrap();
    assert!(
        view.rows.is_empty(),
        "context must not be inferred from decoded rows"
    );
    let ctx = view
        .current_context
        .as_ref()
        .expect("fresh audio authority");
    assert_eq!(ctx.worker_instance, "fixture");
    assert_eq!(ctx.worker_epoch, 1);
    assert_eq!(ctx.server_instance, "worker-fixture");
    assert_eq!(ctx.workspace_id, "worker-workspace");
    assert_eq!(Some(&ctx.route), view.route.as_ref());
    assert_eq!(Some(ctx.decoder_mode), view.mode);
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(json["current_context"]["worker_epoch"], 1);
    assert_eq!(json["current_context"]["decoder_mode"], "FT8");
    assert_eq!(json["current_context"]["route"], json["route"]);
    println!(
        "XNEXUS_RX_CONTEXT_FIXTURE {}",
        serde_json::to_string(&view).unwrap()
    );
    {
        let mut state = worker.state.lock().unwrap();
        state.reset("waiting", "synthetic gap");
    }
    assert!(worker.view().unwrap().current_context.is_none());
    assert!(worker.state.lock().unwrap().view.current_context.is_none());
    bridge.disconnect();
    assert!(worker.view().unwrap().current_context.is_none());
    worker.stop();
    assert!(worker.view().unwrap().current_context.is_none());
}

#[test]
fn receive_context_has_no_authority_after_ordinary_expiry_or_without_mode() {
    let (_bridge, _audio, batches) = authority_batches(48000, &[100]);
    let batch = batches[0].clone();
    let worker = test_worker(state(batch.clone()));
    assert!(worker.view().unwrap().current_context.is_some());
    worker.state.lock().unwrap().view.mode = None;
    assert!(worker.view().unwrap().current_context.is_none());
    worker.state.lock().unwrap().view.mode = Some(Mode::FT8);
    let deadline = Instant::now() + Duration::from_millis(300);
    while batch.is_fresh() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!batch.is_fresh());
    let view = worker.view().unwrap();
    assert!(view.current_context.is_none());
    assert!(view.route.is_none());
}
