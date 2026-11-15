// SPDX-License-Identifier: GPL-3.0-only
//! Acceptance-only ordinary-poll driver. Actual owner cache/mailbox, synthetic classifier.
//! No query wakes the poller and no cache read creates classification evidence.
use crate::{
    companion_needs::{self, Mailbox, QueryCache},
    synthetic_authority, synthetic_needs,
};
use serde_json::{json, Value};
use std::{
    io::Write,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

struct State {
    mailbox: Mailbox,
    cache: QueryCache,
    metadata: Value,
    ready: bool,
    generation: u64,
    poll: u64,
}
pub struct ScheduledNeeds {
    state: Arc<Mutex<State>>,
    stop_marker: PathBuf,
    events: PathBuf,
    stop: mpsc::Sender<()>,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
}
fn event(path: &PathBuf, value: Value) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    writeln!(file, "{value}").unwrap();
}
fn gate(state: &mut State, ready: bool) {
    if state.ready != ready {
        state.ready = ready;
        state.generation = state.generation.checked_add(1).unwrap();
        state.cache.clear();
    }
}
impl ScheduledNeeds {
    pub fn start(root: PathBuf) -> Self {
        let stop_marker = root.join("STOP_NEEDS");
        let events = root.join("needs-cache-events.jsonl");
        let state = Arc::new(Mutex::new(State {
            mailbox: Mailbox::default(),
            cache: QueryCache::default(),
            metadata: json!({"generated_at_unix_ms":companion_needs::now_ms(),"max_age_ms":2000,
                "readiness":"ready","authority":synthetic_authority()}),
            ready: true,
            generation: 0,
            poll: 0,
        }));
        let (stop, rx) = mpsc::channel();
        let shared = state.clone();
        let marker = stop_marker.clone();
        let log = events.clone();
        let thread = thread::spawn(move || {
            // This wait is deliberately independent of HTTP and decoder cadence.
            while rx.recv_timeout(Duration::from_millis(1738))
                == Err(mpsc::RecvTimeoutError::Timeout)
            {
                let mut state = shared.lock().unwrap();
                gate(&mut state, !marker.exists());
                state.poll = state.poll.checked_add(1).unwrap();
                let mut request = json!({"id":format!("ordinary-{}",state.poll),"method":"cluster_snapshot","params":{}});
                state.mailbox.attach(&mut request);
                let generated = companion_needs::now_ms();
                state.metadata = json!({"generated_at_unix_ms":generated,"max_age_ms":2000,
                    "readiness":if state.ready {"ready"} else {"unavailable"},
                    "authority":if state.ready {synthetic_authority()} else {Value::Null}});
                let mut response = json!({"id":request["id"],"result":{}});
                if let Some(query) = request["params"].get("needs_query") {
                    let mut result = synthetic_needs(query, state.ready);
                    result["generated_at_unix_ms"] = json!(generated);
                    // Same correlated wrapper emitted by the actual Swift worker;
                    // result facts here remain explicitly synthetic.
                    response["result"]["needs_reply"] = json!({"batch_id":query["batch_id"],
                        "workspace_id":query["workspace_id"],"result":result});
                    event(
                        &log,
                        json!({"kind":"classified_on_ordinary_poll","request":request,"response":response}),
                    );
                }
                state.mailbox.complete(&mut response);
                assert!(response["result"].get("needs_reply").is_none());
            }
            shared.lock().unwrap().mailbox.retired();
        });
        Self {
            state,
            stop_marker,
            events,
            stop,
            thread: Mutex::new(Some(thread)),
        }
    }
    pub fn metadata(&self) -> Value {
        let mut state = self.state.lock().unwrap();
        gate(&mut state, !self.stop_marker.exists());
        if !state.ready {
            return json!({"generated_at_unix_ms":companion_needs::now_ms(),"max_age_ms":2000,
                "readiness":"unavailable","authority":null});
        }
        state.metadata.clone()
    }
    pub fn query(&self, batch: &Value) -> Result<Value, String> {
        let batch = companion_needs::parse(&serde_json::to_vec(batch).unwrap())?;
        let mut state = self.state.lock().unwrap();
        gate(&mut state, !self.stop_marker.exists());
        if !state.ready {
            return Ok(companion_needs::unknown(&batch, "unavailable"));
        }
        let context = json!({"workspace":"xnexus-disposable-ui-fixture","generation":state.generation,
            "authority":synthetic_authority()});
        let State { cache, mailbox, .. } = &mut *state;
        let answer = cache.query(&batch, context, |batch| {
            mailbox.enqueue(batch, "xnexus-disposable-ui-fixture".into())
        });
        event(
            &self.events,
            json!({"kind":"query","at_unix_ms":companion_needs::now_ms(),
            "queries":batch.queries,"status":if answer.is_ok(){200}else{503},
            "answer":answer.as_ref().ok(),"error":answer.as_ref().err()}),
        );
        answer
    }
    pub fn stop(&self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.lock().unwrap().take() {
            thread.join().unwrap();
        }
    }
}
impl Drop for ScheduledNeeds {
    fn drop(&mut self) {
        self.stop();
    }
}
