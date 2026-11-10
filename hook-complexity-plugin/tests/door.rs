// SPDX-License-Identifier: Apache-2.0
//! Linked and dropped-in configuration and unsupported-projection behavior through the real loader.
use std::sync::Arc;
mod support;

use busbar_contract::abi::hook::{slot, ConfigureIn, ConfigureOut, DecideOut};
use busbar_contract::abi::host::hook::{DecideFrame, DecideView};
use busbar_contract::abi::mechanism::call::{AbiStr, Blob, Outcome, BLOB_JSON};
use busbar_contract::abi::mechanism::lifecycle::{slot as life, OpenIn, OpenOut, ReleaseIn};
use busbar_contract::abi::sdk::door::blank_out;
use busbar_contract::hooks::{Candidate, RoutingContext, RoutingRequest};
use busbar_contract::SignalBag;
use busbar_plugin_loader::dispatch::kinds::hook::Hook;
use busbar_plugin_loader::dispatch::{
    in_head, load_dropped, load_linked, out_head, rendering_of, Bind, ConnTable, DispatchConfig,
    Dispatcher, Frame, LinkedRow, NoSink, Plugin,
};

fn blob(text: &str) -> Blob {
    Blob {
        ptr: text.as_ptr(),
        len: text.len(),
        fmt: BLOB_JSON,
        flags: 0,
    }
}

/// One door, bound with a recording sink.
struct Bound {
    plugin: Plugin<Hook>,
    _dispatcher: Arc<Dispatcher>,
}

/// The linked door (`false`) or the dropped-in cdylib (`true`), bound.
fn bind(dropped: bool) -> Bound {
    let dispatcher = Arc::new(Dispatcher::with_services(
        DispatchConfig::default(),
        Arc::new(support::NoGeneralServices),
        busbar_contract::services::test_caller_binding(),
    ));
    let bind = Bind {
        instance: Arc::from("complexity"),
        max_inflight_cap: 64,
        sink: Arc::new(NoSink),
        dispatcher: dispatcher.adopter(),
        conns: ConnTable::NoNeeds,
    };
    let plugin = if dropped {
        let path = {
            let path = std::env::var_os("BUSBAR_PLUGIN_CDYLIB")
                .map(std::path::PathBuf::from)
                .expect("set BUSBAR_PLUGIN_CDYLIB to the freshly built complexity cdylib");
            assert!(
                path.is_absolute() && path.is_file(),
                "actual complexity cdylib required: {}",
                path.display()
            );
            path
        };
        let stated = rendering_of(busbar_hook_complexity::linked::door)
            .expect("the door renders its Statement");
        load_dropped::<Hook>(&path, &stated, bind)
    } else {
        load_linked::<Hook>(
            &LinkedRow::of(busbar_hook_complexity::linked::door).expect("the door states itself"),
            bind,
        )
    }
    .expect("the door loads");
    Bound {
        plugin,
        _dispatcher: dispatcher,
    }
}

impl Bound {
    fn open(&self, settings: &str) -> Outcome {
        let mut f = Frame::new(
            OpenIn {
                head: in_head(),
                host: std::ptr::null(),
                settings: blob(settings),
                secrets: std::ptr::null(),
                secrets_len: 0,
                generation: 1,
                err_buf: std::ptr::null_mut(),
                err_cap: 0,
            },
            OpenOut {
                head: out_head(),
                instance: std::ptr::null_mut(),
                err_len: 0,
            },
        );
        self.plugin.call(life::OPEN, &mut f).outcome
    }

    fn configure_snapshot(
        &self,
        settings: &str,
        version: u64,
    ) -> (Outcome, u64, Option<(u64, u64, serde_json::Value)>) {
        let mut f = Frame::new(
            ConfigureIn {
                head: in_head(),
                version,
                settings: blob(settings),
                name: AbiStr {
                    ptr: std::ptr::null(),
                    len: 0,
                },
            },
            blank_out::<ConfigureOut>(),
        );
        let called = self.plugin.call(slot::CONFIGURE, &mut f);
        let blob = f.out.head.extensions;
        let bytes = if blob.ptr.is_null() || blob.len == 0 {
            Vec::new()
        } else {
            // SAFETY: the validated reply's lease remains live until the explicit RELEASE below.
            unsafe { std::slice::from_raw_parts(blob.ptr, blob.len) }.to_vec()
        };
        if called.lease != 0 {
            let mut release = Frame::new(
                ReleaseIn {
                    head: in_head(),
                    lease: called.lease,
                },
                out_head(),
            );
            assert_eq!(
                self.plugin.call(life::RELEASE, &mut release).outcome,
                Outcome::Ready
            );
        }
        let snapshot = busbar_contract::abi::mechanism::extensions::get(
            &bytes,
            "hook.configure_checkpoint.v1",
        )
        .map(|value| {
            assert!(value.len() > 16);
            let revision = u64::from_le_bytes(value[..8].try_into().unwrap());
            let version = u64::from_le_bytes(value[8..16].try_into().unwrap());
            let settings = serde_json::from_slice(&value[16..]).expect("complete JSON settings");
            (revision, version, settings)
        });
        (called.outcome, f.out.acked_version, snapshot)
    }

    /// One `decide` over two candidates, idx 0 cheaper and idx 1 faster: the order written.
    fn request_frame(complete: bool) -> Arc<DecideFrame> {
        let request = RoutingRequest {
            request_id: 1,
            pool: "p",
            ingress_protocol: "wire-a",
            requested_model: None,
            message_count: 1,
            tool_count: 0,
            has_tools: false,
            total_chars: 10,
            system_chars: 0,
            max_tokens: None,
            stream: false,
            prompt: complete.then(|| busbar_contract::hooks::PromptProjection {
                system: None,
                messages: vec![("user".into(), "hello".into())],
            }),
            identity: None,
            signals: {
                let mut signals = SignalBag::new();
                if complete {
                    signals.push(
                        busbar_contract::signal::Signal::RequestTextComplete,
                        busbar_contract::signal::SignalValue::Bool(true),
                    );
                }
                signals
            },
            session: None,
        };
        let candidate = |idx, cost, latency| Candidate {
            idx,
            model: if idx == 0 { "small" } else { "large" },
            provider: "p",
            weight: 1,
            context_max: None,
            tier: None,
            cost_per_mtok: Some(cost),
            tags: &[],
            latency_ms: Some(latency),
            available_concurrency: 1,
            budget_remaining: None,
            rate_headroom: None,
            signals: {
                use busbar_contract::signal::route_cost::{append, Term};
                let mut signals = SignalBag::new();
                append(
                    &mut signals,
                    [Term {
                        class: "work",
                        price_per_unit: Some(cost),
                        expected_units: 1.0,
                    }],
                );
                signals
            },
        };
        let candidates = [candidate(0, 1.0, 900.0), candidate(1, 9.0, 10.0)];
        let ctx = RoutingContext {
            pool: "p",
            budget_remaining: None,
            budget: &[],
        };
        DecideFrame::first(DecideView::build(&request, &candidates, &ctx))
    }
    fn abstains(&self, complete: bool) {
        let frame = Self::request_frame(complete);
        let mut f = Frame::new(frame.input(), blank_out::<DecideOut>());
        assert_eq!(
            self.plugin.call(slot::DECIDE, &mut f).outcome,
            Outcome::Ready
        );
        assert_eq!(f.out.verbs, busbar_contract::abi::hook::VERB_ABSTAIN);
        assert_eq!(f.out.order_written, 0);
    }
}

const SETTINGS: &str = r#"{"classifier_model":"classifier","tiers":[{"label":"easy","description":"simple","candidates":["small"]},{"label":"hard","description":"complex","candidates":["large"]}]}"#;
#[test]
fn classification_door_replays_immutable_settings_and_abstains_without_projection() {
    for dropped in [false, true] {
        let bound = bind(dropped);
        assert_eq!(bound.open(SETTINGS), Outcome::Ready);
        let replay = bound.configure_snapshot(SETTINGS, 2);
        assert_eq!(replay.0, Outcome::Ready);
        let (revision, version, checkpoint) = replay.2.unwrap();
        assert_eq!((revision, version), (1, 2));
        assert_eq!(checkpoint["min_confidence"], 0.7);
        assert_eq!(checkpoint["missing_confidence"], "abstain");
        let again = bound.configure_snapshot(SETTINGS, 2).2.unwrap();
        assert_eq!((again.0, again.1), (2, 2));
        let stale = bound.configure_snapshot(SETTINGS, 1).2.unwrap();
        assert_eq!((stale.0, stale.1), (2, 2));
        let mut changed = checkpoint.clone();
        changed["min_confidence"] = serde_json::json!(0.9);
        assert_eq!(
            bound.configure_snapshot(&changed.to_string(), 3).0,
            Outcome::Failed
        );
        bound.abstains(false);
        bound.abstains(true);
        let restored = bind(dropped);
        assert_eq!(restored.open(&checkpoint.to_string()), Outcome::Ready);
        assert_eq!(
            restored.configure_snapshot(&checkpoint.to_string(), 2).0,
            Outcome::Ready
        );
        restored.abstains(false);
        restored.abstains(true);
    }
}

#[derive(Debug, Default)]
struct PendingClassification(std::sync::atomic::AtomicUsize);
impl busbar_contract::services::HookChildInvocation for PendingClassification {
    fn request(
        &self,
        caller: &busbar_contract::services::Caller,
        payload: Vec<u8>,
        later: busbar_contract::services::Later,
    ) -> busbar_contract::services::Ran {
        assert_eq!(
            caller.kind(),
            busbar_contract::abi::mechanism::KindCode::Hook
        );
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let request: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(request["model"], "classifier");
        assert_eq!(
            request["messages"],
            serde_json::json!([{"role":"user","text":"hello"}])
        );
        let mut answer = busbar_contract::services::Stored::ready(200);
        answer.bytes=br#"{"version":1,"label":"easy","confidence":0.8,"source":"jev","schema":"jev-choice-v1"}"#.to_vec();
        std::thread::spawn(move || {
            later(answer);
        });
        busbar_contract::services::Ran::Later
    }
}
#[tokio::test]
async fn classification_child_parks_and_resumes_once_through_both_doors() {
    use busbar_contract::hook_calls::{Answered, HookCalls};
    for dropped in [false, true] {
        let bound = bind(dropped);
        let hook = busbar_plugin_loader::hook_door::HookInstance::open(
            bound.plugin,
            bound._dispatcher,
            SETTINGS.as_bytes(),
            || Err("unexpected rebind".into()),
        )
        .unwrap();
        let child = Arc::new(PendingClassification::default());
        let frame = Bound::request_frame(true).with_child(Some(child.clone()));
        match hook.decide(frame, std::time::Duration::from_secs(2)).await {
            Answered::Answer {
                outcome,
                out,
                frame,
                ..
            } => {
                assert_eq!(outcome, Outcome::Ready);
                assert_eq!(
                    out.verbs,
                    busbar_contract::abi::hook::VERB_PRIORITIZE,
                    "dropped={dropped} child calls={}",
                    child.0.load(std::sync::atomic::Ordering::SeqCst)
                );
                assert_eq!(frame.order(out.order_written), vec![0]);
            }
            other => panic!("classification did not complete: {other:?}"),
        }
        assert_eq!(
            child.0.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "one child service submission despite pending/resume"
        );
    }
}
