// SPDX-License-Identifier: GPL-3.0-only
//! Acceptance executable, no GUI or device access. Compiles actual native worker.
include!(env!("XNEXUS_UI_WORKER_MODULES"));
use std::{fs, path::PathBuf, process::{Child, Command, Stdio}, time::{Duration, Instant}};
use serde_json::{json, Value};
use xnexus_xlog_client::{BridgeClient, NeedStatus};
use xlog_rx_worker::{Mode, Worker};
struct Fixture { root: PathBuf, child: Child }
impl Fixture {
    fn launch(kind: &str, malformed: bool) -> Self {
        let id=uuid::Uuid::new_v4(); let root=std::env::temp_dir().join(format!("xnexus-ui-{id}"));
        fs::create_dir(&root).unwrap();
        fs::write(root.join(".xnexus-ui-fixture.json"),serde_json::to_vec(&json!({"schema":1,"id":id.to_string(),"synthetic":true,"synthetic_audio":kind})).unwrap()).unwrap();
        if kind=="generated-ft4" {
            if malformed {fs::write(root.join("generated-ft4.s16le"),[0;4]).unwrap();}
            else {fs::copy(std::env::var_os("XNEXUS_UI_WAVE").unwrap(),root.join("generated-ft4.s16le")).unwrap();}
        }
        let child=Command::new(std::env::var_os("XNEXUS_UI_OWNER").unwrap())
            .arg(format!("--fixture-root={}",root.display())).current_dir(&root)
            .stdout(Stdio::from(fs::File::create(root.join("stdout.log")).unwrap()))
            .stderr(Stdio::from(fs::File::create(root.join("stderr.log")).unwrap())).spawn().unwrap();
        Self{root,child}
    }
    fn pair(&mut self) -> BridgeClient {
        let deadline=Instant::now()+Duration::from_secs(5);
        loop {
            assert!(self.child.try_wait().unwrap().is_none(),"fixture exited: {}",self.root.display());
            if self.root.join("owner.json").exists() {break;}
            assert!(Instant::now()<deadline,"fixture readiness timeout");std::thread::sleep(Duration::from_millis(20));
        }
        let identity:Value=serde_json::from_slice(&fs::read(self.root.join("owner.json")).unwrap()).unwrap();
        assert_eq!(identity["pid"],self.child.id());assert_eq!(identity["synthetic"],true);
        let pair:Value=serde_json::from_slice(&fs::read(self.root.join("pairing.json")).unwrap()).unwrap();
        let mut client=BridgeClient::new(pair["endpoint"].as_str().unwrap(),pair["token"].as_str().unwrap()).unwrap();
        client.connect().unwrap();client
    }
    fn finish(&mut self) {
        fs::write(self.root.join("STOP"),[]).unwrap();
        let deadline=Instant::now()+Duration::from_secs(3);
        loop { if let Some(status)=self.child.try_wait().unwrap(){assert!(status.success());break;}
            assert!(Instant::now()<deadline,"fixture did not close normally");std::thread::sleep(Duration::from_millis(20)); }
    }
}
impl Drop for Fixture { fn drop(&mut self) {let _=fs::write(self.root.join("STOP"),[]);} }
fn main() {
    let evidence=PathBuf::from(std::env::var_os("XNEXUS_UI_EVIDENCE").unwrap());
    let mut bad=Fixture::launch("generated-ft4",true);
    let until=Instant::now()+Duration::from_secs(3);
    loop {if let Some(status)=bad.child.try_wait().unwrap(){assert!(!status.success());break;}
        assert!(Instant::now()<until);std::thread::sleep(Duration::from_millis(20));}
    assert!(!bad.root.join("owner.json").exists());
    let mut silence=Fixture::launch("silence48k",false);let client=silence.pair();let mut audio=client.audio_handle().unwrap();
    let until=Instant::now()+Duration::from_secs(3);
    loop { let batch=audio.poll().unwrap();if !batch.chunks.is_empty(){assert!(batch.chunks.iter().flat_map(|c|&c.samples).all(|s|*s==0));break;}
        assert!(Instant::now()<until);std::thread::sleep(Duration::from_millis(20)); }
    silence.finish();
    let mut owner=Fixture::launch("generated-ft4",false);
    fs::write(evidence.join("fixture-roots.json"),serde_json::to_vec_pretty(&json!({"malformed":bad.root,"silence":silence.root,"generated":owner.root})).unwrap()).unwrap();
    let mut client=owner.pair();client.poll().unwrap();
    let hello=client.hello().unwrap().clone();
    let worker=Worker::start_with_needs(client.audio_handle().unwrap(),hello.server_instance,hello.workspace_id,Mode::FT4,Some(client.needs_handle().unwrap())).unwrap();
    let started=Instant::now();let mut next_poll=Instant::now();let mut phase=0;let mut decode_id=String::new();let mut old_generated=None;let mut timings=Vec::new();
    let mut last_sequence=0;let mut observer=client.audio_handle().unwrap();
    let mut previous_context=None; let mut new_epoch=None; let mut recovery=0;
    while phase<5 {
        assert!(started.elapsed()<Duration::from_secs(75),"decode/needs/gap timeout phase{phase}: {}",worker.view().unwrap().reason);
        if Instant::now()>=next_poll {
            let snapshot=client.poll().unwrap();
            if let Some(needs)=snapshot.needs.as_ref(){worker.needs_snapshot_observed(needs.generated_at_unix_ms);}
            fs::write(evidence.join("last-native-view.json"),serde_json::to_vec_pretty(&worker.view().unwrap()).unwrap()).unwrap();
            next_poll=Instant::now()+Duration::from_millis(500);
        }
        let view=worker.view().unwrap(); assert!(view.enabled);
        if phase==0 {if let Some(row)=view.rows.iter().find(|r|r.message=="CQ K1ABC FN42" && r.needs.status==NeedStatus::Needed){
            assert_eq!(row.sender_callsign.as_deref(),Some("K1ABC"));assert_eq!(row.needs.score,Some(100));
            assert!(row.needs.reasons.iter().any(|r|r.contains("Synthetic")));
            decode_id=row.decode_id.clone();old_generated=row.needs.generated_at_unix_ms;
            timings.push(json!({"phase":"known","elapsed_ms":started.elapsed().as_millis(),"row":row}));
            fs::write(owner.root.join("STOP_NEEDS"),[]).unwrap();
            last_sequence=observer.poll().unwrap().chunks.last().unwrap().sequence; phase=1;
        }} else if phase==1 {if let Some(row)=view.rows.iter().find(|r|r.decode_id==decode_id && r.needs.status==NeedStatus::Unknown){
            let batch=observer.poll().unwrap();
            if batch.chunks.last().is_some_and(|c|c.sequence>last_sequence){
                timings.push(json!({"phase":"unknown_pcm_continues","elapsed_ms":started.elapsed().as_millis(),"row":row}));
                fs::remove_file(owner.root.join("STOP_NEEDS")).unwrap();phase=2;
            }
        }} else if phase==2 {if let Some(row)=view.rows.iter().find(|r|r.decode_id==decode_id && r.needs.status==NeedStatus::Needed && r.needs.generated_at_unix_ms>old_generated){
            timings.push(json!({"phase":"new_known","elapsed_ms":started.elapsed().as_millis(),"row":row}));
            previous_context=Some(row.context.clone());
            old_generated=row.needs.generated_at_unix_ms;
            fs::write(owner.root.join("GAP_REQUEST"),[]).unwrap(); phase=3;
        }} else if phase==3 && owner.root.join(if recovery==0 {"GAP_ACK.json"} else {"TIMING_ACK.json"}).exists() {
            let gap:Value=serde_json::from_slice(&fs::read(owner.root.join(if recovery==0 {"GAP_ACK.json"} else {"TIMING_ACK.json"})).unwrap()).unwrap();
            let previous=previous_context.as_ref().unwrap();
            assert_eq!(gap["old_descriptor"]["epoch"],previous.route.epoch);
            assert_eq!(gap["old_descriptor"]["capture_id"],gap["new_descriptor"]["capture_id"]);
            assert_ne!(gap["old_descriptor"]["epoch"],gap["new_descriptor"]["epoch"]);
            assert_eq!(gap["waiting"]["state"],"stopped");
            assert!(gap["waiting"]["descriptor"].is_null());
            assert_eq!(gap["waiting"]["chunks"],json!([]));
            if view.rows.is_empty() {
                new_epoch=Some(gap["new_descriptor"]["epoch"].as_str().unwrap().to_string());
                timings.push(json!({"phase":"recovery_revoked_rows_and_needs","recovery":recovery,"elapsed_ms":started.elapsed().as_millis(),"view":view,"gap":gap}));
                phase=4;
            }
        } else if phase==4 {
            assert!(view.rows.iter().all(|row| row.decode_id!=decode_id && row.context.route.epoch==*new_epoch.as_ref().unwrap()),"old epoch row restored after gap observation");
            if let Some(row)=view.rows.iter().find(|r|r.message=="CQ K1ABC FN42" && r.needs.status==NeedStatus::Needed) {
                assert_ne!(&row.context,previous_context.as_ref().unwrap());
                assert_eq!(row.context.route.capture_id,previous_context.as_ref().unwrap().route.capture_id);
                assert!(row.needs.generated_at_unix_ms>old_generated,"new epoch needs must come from a fresh query");
                timings.push(json!({"phase":"new_epoch_complete_period_known","recovery":recovery,"elapsed_ms":started.elapsed().as_millis(),"row":row}));
                if recovery==0 {
                    recovery=1; previous_context=Some(row.context.clone()); decode_id=row.decode_id.clone();
                    old_generated=row.needs.generated_at_unix_ms;
                    fs::write(owner.root.join("TIMING_REQUEST"),[]).unwrap(); phase=3;
                } else { phase=5; }
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    worker.stop();assert!(!worker.view().unwrap().enabled);assert!(worker.view().unwrap().rows.is_empty());
    drop(worker);owner.finish();
    fs::write(evidence.join("headless.json"),serde_json::to_vec_pretty(&json!({"passed":true,"actual_decode":true,"synthetic_classification":true,"hardware":false,"gui":false,"malformed_root":bad.root,"silence_root":silence.root,"generated_root":owner.root,"timings":timings,"normal_stop":true})).unwrap()).unwrap();
    println!("PASS malformed waveform, silence, actual FT4 decode -> Needed -> Unknown with PCM progress -> fresh Needed -> declared gap/empty -> new-epoch decoded Needed -> timing-overflow discard/empty -> new-epoch decoded Needed -> Stop -> normal fixture exit");
}
