// SPDX-License-Identifier: GPL-3.0-only
//! OM0RX-xNexus: what to do when the antenna tuner (KAT500) or the amplifier (KPA500) faults while the
//! autopilot runs (operator, 2026-11-25: the autopilot "should pause", and then "TX needs to stop
//! first, then clear fault, then tune, wait for tune, then align just in case and only then TX
//! enabled", for the KAT as well as the KPA).
//!
//! The devices belong to the OM0RX Station Agent on the same PC, which enforces everything on the air
//! itself: it clears faults in receive and Standby only, tunes with its own low-power carrier and
//! checks the SWR. This is only the order of the steps, pure and without any I/O: the shell asks the
//! Agent for its state about once a second, hands it to [`Guard::poll`], and does what comes back. Each
//! step waits for the one before to be confirmed. One recovery per fault: a fault again within
//! [`AGAIN_SECS`], a failed step or a step out of time leaves the station paused for the operator, as
//! does the operator touching transmit meanwhile ([`Guard::operator`]).

use serde_json::Value;

/// A fault this soon after a recovery is left to the operator, seconds.
const AGAIN_SECS: u64 = 600;
/// How long the radio may take to be back in receive, seconds.
const RECEIVE_SECS: u64 = 20;
/// Receive is waited for at least this long after transmit was stopped, seconds.
const SETTLE_SECS: u64 = 2;
/// How long one step may take, seconds. A tune is given up by the Agent itself after 30.
const STEP_SECS: u64 = 30;
const TUNE_SECS: u64 = 90;

/// What the Agent's state says about the devices.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StationView {
    /// The tuner's and the amplifier's fault, when there is one.
    pub kat_fault: Option<String>,
    pub kpa_fault: Option<String>,
    /// The radio is known to be receiving.
    pub receiving: bool,
    /// The route (band, antenna) is confirmed on both devices.
    pub aligned: bool,
    /// The last station transaction: its id, kind and state, and the error it ended with.
    pub transaction: Option<Transaction>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Transaction {
    pub id: String,
    pub kind: String,
    pub state: String,
    pub error: String,
}

impl StationView {
    /// From the Agent's `GET /api/v1/state`. `None` when it carries no devices.
    pub fn from_state(v: &Value) -> Option<Self> {
        let devices = v.get("elecraft")?;
        let fault = |name: &str| {
            let d = devices.get(name)?;
            if d.get("connected").and_then(Value::as_bool) != Some(true) {
                return None;
            }
            let f = match d.get("fault")? {
                Value::String(s) => s.trim().to_string(),
                Value::Number(n) => n.to_string(),
                _ => return None,
            };
            (!f.trim_matches('0').is_empty()).then_some(f)
        };
        let station = v.get("station");
        let path = |keys: &[&str]| keys.iter().try_fold(station?, |v, k| v.get(k));
        let text = |v: Option<&Value>| v.and_then(Value::as_str).unwrap_or("").to_string();
        Some(Self {
            kat_fault: fault("kat500"),
            kpa_fault: fault("kpa500"),
            receiving: v.pointer("/radio/mox").and_then(Value::as_bool) == Some(false),
            aligned: path(&["path", "alignment"]).and_then(Value::as_str) == Some("confirmed"),
            transaction: path(&["transaction"])
                .filter(|t| t.is_object())
                .map(|t| Transaction {
                    id: text(t.get("id")),
                    kind: text(t.get("kind")),
                    state: text(t.get("state")),
                    error: text(t.pointer("/error/message")),
                }),
        })
    }

    /// The faults in words, or `None`.
    pub fn fault(&self) -> Option<String> {
        let one = |name: &str, f: &Option<String>| f.as_ref().map(|f| format!("{name} fault {f}"));
        let list: Vec<String> = [
            one("KAT500", &self.kat_fault),
            one("KPA500", &self.kpa_fault),
        ]
        .into_iter()
        .flatten()
        .collect();
        (!list.is_empty()).then(|| list.join(" and "))
    }
}

/// What the shell does next.
#[derive(Debug, Clone, PartialEq)]
pub enum Act {
    Nothing,
    /// Stop transmit and pause the autopilot, for this reason.
    Hold(String),
    /// Send the Agent this command, and hand its answer to [`Guard::reply`].
    Send(&'static str),
    /// Transmit and the autopilot on again.
    Resume,
    /// Left paused for the operator, for this reason.
    GiveUp(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
enum Step {
    #[default]
    Watching,
    /// Transmit stopped; waiting for receive.
    Stopped,
    /// Waiting for the answer to a command, or for what it does to show.
    Clearing,
    Tuning,
    Aligning,
    /// Left to the operator.
    GivenUp,
}

#[derive(Debug, Default)]
pub struct Guard {
    step: Step,
    /// When this step began, Unix seconds.
    since: u64,
    /// The answer to the step's command has come.
    answered: bool,
    /// The tune's transaction.
    tune_id: String,
    /// When the last recovery succeeded, Unix seconds.
    recovered: Option<u64>,
    /// The operator went on despite the fault now standing.
    theirs: bool,
}

impl Guard {
    /// The autopilot is off: watch afresh.
    pub fn reset(&mut self) {
        self.step = Step::Watching;
        self.theirs = false;
    }

    /// The operator took transmit in hand: the fault now standing is theirs, the next one is not.
    pub fn operator(&mut self) {
        self.step = Step::Watching;
        self.theirs = true;
    }

    /// Is a recovery under way, or the station left to the operator?
    pub fn busy(&self) -> bool {
        self.step != Step::Watching
    }

    fn enter(&mut self, step: Step, now: u64) {
        self.step = step;
        self.since = now;
        self.answered = false;
    }

    fn give_up(&mut self, why: String, now: u64) -> Act {
        self.enter(Step::GivenUp, now);
        Act::GiveUp(why)
    }

    /// The Agent's answer to the command sent: the id of the transaction it started, or its error.
    pub fn reply(&mut self, answer: Result<String, String>, now: u64) -> Act {
        match answer {
            Ok(id) => {
                self.answered = true;
                self.tune_id = id;
                Act::Nothing
            }
            Err(e) => {
                let step = match self.step {
                    Step::Clearing => "the faults were not cleared",
                    Step::Tuning => "the tune did not start",
                    Step::Aligning => "the station was not aligned",
                    _ => return Act::Nothing,
                };
                self.give_up(format!("{step}: {e}"), now)
            }
        }
    }

    /// One look at the Agent's state. `recover`: the operator switched the recovery on.
    pub fn poll(&mut self, view: Option<&StationView>, recover: bool, now: u64) -> Act {
        let waited = now.saturating_sub(self.since);
        let Some(view) = view else {
            // The Agent does not answer: no fault is known, and a recovery cannot go on.
            return match self.step {
                Step::Watching | Step::GivenUp => Act::Nothing,
                _ if waited <= RECEIVE_SECS => Act::Nothing,
                _ => self.give_up("the Station Agent does not answer".into(), now),
            };
        };
        let fault = view.fault();
        match self.step {
            Step::Watching => match fault {
                Some(_) if self.theirs => Act::Nothing,
                Some(f) => {
                    self.enter(Step::Stopped, now);
                    Act::Hold(f)
                }
                None => {
                    self.theirs = false;
                    Act::Nothing
                }
            },
            Step::Stopped => {
                let f = fault.unwrap_or_else(|| "the fault is gone".into());
                if !recover {
                    self.give_up(f, now)
                } else if self
                    .recovered
                    .is_some_and(|t| now.saturating_sub(t) < AGAIN_SECS)
                {
                    self.give_up(format!("{f} again soon after a recovery"), now)
                } else if view.receiving && waited >= SETTLE_SECS {
                    self.enter(Step::Clearing, now);
                    Act::Send("station_clear_faults")
                } else if waited > RECEIVE_SECS {
                    self.give_up(format!("{f}, and the radio is not in receive"), now)
                } else {
                    Act::Nothing
                }
            }
            Step::Clearing => match (self.answered, fault) {
                (true, None) => {
                    self.enter(Step::Tuning, now);
                    self.tune_id.clear();
                    Act::Send("station_tune")
                }
                (_, f) if waited > STEP_SECS => self.give_up(
                    f.unwrap_or_else(|| "no answer to the fault clear".into()),
                    now,
                ),
                _ => Act::Nothing,
            },
            Step::Tuning => {
                let t = view
                    .transaction
                    .as_ref()
                    .filter(|t| self.answered && t.kind == "tune" && t.id == self.tune_id);
                match t.map(|t| (t.state.as_str(), t.error.clone())) {
                    Some(("succeeded", _)) => match fault {
                        None => {
                            self.enter(Step::Aligning, now);
                            Act::Send("station_prepare_route")
                        }
                        Some(f) => self.give_up(f, now),
                    },
                    Some(("failed" | "cancelled", e)) => {
                        self.give_up(format!("the tune failed: {e}"), now)
                    }
                    _ if waited > TUNE_SECS => self.give_up("the tune did not finish".into(), now),
                    _ => Act::Nothing,
                }
            }
            Step::Aligning => match fault {
                Some(f) => self.give_up(f, now),
                None if self.answered && view.aligned && view.receiving => {
                    self.enter(Step::Watching, now);
                    self.recovered = Some(now);
                    Act::Resume
                }
                None if waited > STEP_SECS => {
                    self.give_up("the station was not aligned".into(), now)
                }
                None => Act::Nothing,
            },
            Step::GivenUp => Act::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn state(kat: &str, kpa: &str, mox: bool) -> Value {
        json!({
            "radio": {"mox": mox},
            "elecraft": {
                "kat500": {"enabled": true, "connected": true, "fault": kat, "swr": 1.2},
                "kpa500": {"enabled": true, "connected": true, "fault": kpa, "operate": false},
            },
            "station": {"path": {"alignment": "confirmed"}, "transaction": null},
        })
    }
    fn view(kat: &str, kpa: &str) -> StationView {
        StationView::from_state(&state(kat, kpa, false)).unwrap()
    }
    fn tune(state: &str, error: &str) -> StationView {
        StationView {
            transaction: Some(Transaction {
                id: "t1".into(),
                kind: "tune".into(),
                state: state.into(),
                error: error.into(),
            }),
            ..view("0", "00")
        }
    }
    /// Up to the tune under way, at second 10.
    fn tuning() -> Guard {
        let mut g = Guard::default();
        assert_eq!(
            g.poll(Some(&view("2", "00")), true, 0),
            Act::Hold("KAT500 fault 2".into())
        );
        assert_eq!(g.poll(Some(&view("2", "00")), true, 1), Act::Nothing);
        assert_eq!(
            g.poll(Some(&view("2", "00")), true, 3),
            Act::Send("station_clear_faults")
        );
        assert_eq!(
            g.poll(Some(&view("0", "00")), true, 4),
            Act::Nothing,
            "not answered yet"
        );
        assert_eq!(g.reply(Ok(String::new()), 5), Act::Nothing);
        assert_eq!(
            g.poll(Some(&view("2", "00")), true, 6),
            Act::Nothing,
            "still faulted"
        );
        assert_eq!(
            g.poll(Some(&view("0", "00")), true, 7),
            Act::Send("station_tune")
        );
        assert_eq!(g.reply(Ok("t1".into()), 8), Act::Nothing);
        g
    }

    #[test]
    fn both_devices_faults_are_read() {
        assert_eq!(view("0", "00").fault(), None);
        assert_eq!(view("2", "00").fault().as_deref(), Some("KAT500 fault 2"));
        assert_eq!(view("0", "05").fault().as_deref(), Some("KPA500 fault 05"));
        assert_eq!(
            view("3", "05").fault().as_deref(),
            Some("KAT500 fault 3 and KPA500 fault 05")
        );
        let mut off = state("2", "00", false);
        off["elecraft"]["kat500"]["connected"] = json!(false);
        assert_eq!(
            StationView::from_state(&off).unwrap().fault(),
            None,
            "a tuner that is not connected has no known fault"
        );
        assert_eq!(StationView::from_state(&json!({"radio": {}})), None);
        assert!(
            !StationView::from_state(&state("0", "00", true))
                .unwrap()
                .receiving
        );
    }

    #[test]
    fn a_fault_is_cleared_tuned_and_aligned_in_that_order_and_only_then_transmit_is_on() {
        let mut g = tuning();
        assert_eq!(g.poll(Some(&tune("running", "")), true, 12), Act::Nothing);
        assert_eq!(
            g.poll(Some(&tune("succeeded", "")), true, 20),
            Act::Send("station_prepare_route")
        );
        assert_eq!(
            g.poll(Some(&tune("succeeded", "")), true, 21),
            Act::Nothing,
            "aligning not answered"
        );
        assert_eq!(g.reply(Ok(String::new()), 22), Act::Nothing);
        let unaligned = StationView {
            aligned: false,
            ..tune("succeeded", "")
        };
        assert_eq!(g.poll(Some(&unaligned), true, 23), Act::Nothing);
        assert_eq!(g.poll(Some(&tune("succeeded", "")), true, 24), Act::Resume);
        assert!(!g.busy());
    }

    #[test]
    fn a_failed_step_leaves_the_station_paused() {
        let mut g = tuning();
        assert_eq!(
            g.poll(
                Some(&tune("failed", "Tuned SWR 2.10 is above the 1.50 target")),
                true,
                20
            ),
            Act::GiveUp("the tune failed: Tuned SWR 2.10 is above the 1.50 target".into())
        );
        assert_eq!(
            g.poll(Some(&tune("succeeded", "")), true, 30),
            Act::Nothing,
            "and nothing resumes it"
        );

        let mut g = tuning();
        assert_eq!(
            g.poll(Some(&tune("running", "")), true, 99),
            Act::GiveUp("the tune did not finish".into())
        );

        let mut g = Guard::default();
        g.poll(Some(&view("0", "05")), true, 0);
        g.poll(Some(&view("0", "05")), true, 3);
        assert_eq!(
            g.reply(Err("KPA500 powered off".into()), 4),
            Act::GiveUp("the faults were not cleared: KPA500 powered off".into())
        );

        // An older tune's transaction is not this one's.
        let mut g = tuning();
        let mut old = tune("succeeded", "");
        old.transaction.as_mut().unwrap().id = "t0".into();
        assert_eq!(g.poll(Some(&old), true, 12), Act::Nothing);
    }

    #[test]
    fn nothing_is_cleared_while_the_radio_transmits() {
        let mut g = Guard::default();
        let on_air = StationView::from_state(&state("2", "00", true)).unwrap();
        g.poll(Some(&on_air), true, 0);
        assert_eq!(g.poll(Some(&on_air), true, 10), Act::Nothing);
        assert_eq!(
            g.poll(Some(&on_air), true, 21),
            Act::GiveUp("KAT500 fault 2, and the radio is not in receive".into())
        );
    }

    #[test]
    fn without_the_switch_it_only_pauses_and_a_fault_again_is_left_to_the_operator() {
        let mut g = Guard::default();
        g.poll(Some(&view("2", "00")), false, 0);
        assert_eq!(
            g.poll(Some(&view("2", "00")), false, 1),
            Act::GiveUp("KAT500 fault 2".into())
        );
        assert_eq!(
            g.poll(Some(&view("0", "00")), false, 2),
            Act::Nothing,
            "transmit is not put back by itself"
        );
        g.reset();
        assert!(!g.busy());

        let mut g = tuning();
        g.poll(Some(&tune("succeeded", "")), true, 20);
        g.reply(Ok(String::new()), 21);
        assert_eq!(g.poll(Some(&tune("succeeded", "")), true, 22), Act::Resume);
        assert_eq!(
            g.poll(Some(&view("2", "00")), true, 100),
            Act::Hold("KAT500 fault 2".into())
        );
        assert_eq!(
            g.poll(Some(&view("2", "00")), true, 103),
            Act::GiveUp("KAT500 fault 2 again soon after a recovery".into())
        );
    }

    #[test]
    fn a_fault_the_operator_goes_on_with_is_theirs_until_it_has_gone() {
        let mut g = tuning();
        g.operator();
        assert_eq!(g.poll(Some(&view("2", "00")), true, 10), Act::Nothing);
        assert_eq!(g.poll(Some(&view("0", "00")), true, 11), Act::Nothing);
        assert_eq!(
            g.poll(Some(&view("2", "00")), true, 12),
            Act::Hold("KAT500 fault 2".into())
        );
        g.operator();
        g.reset();
        assert_eq!(
            g.poll(Some(&view("2", "00")), true, 13),
            Act::Hold("KAT500 fault 2".into()),
            "the autopilot switched on again watches afresh"
        );
    }

    #[test]
    fn an_agent_that_does_not_answer_is_no_fault_and_no_recovery() {
        let mut g = Guard::default();
        assert_eq!(g.poll(None, true, 0), Act::Nothing);
        let mut g = tuning();
        assert_eq!(g.poll(None, true, 10), Act::Nothing);
        assert_eq!(
            g.poll(None, true, 40),
            Act::GiveUp("the Station Agent does not answer".into())
        );
    }
}
