// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2026 Busbar Inc and contributors

use crate::{config::Config, routing, ALIAS, NAME};
use busbar_contract::abi::hook::{Tail, CLASS_GATE, PROMPT_RO, USER_NO};
use busbar_contract::abi::mechanism::door::{MarkWord, Rewrite, Statement, MARK_WORD_HOOK};
use busbar_contract::abi::mechanism::ticket::CompletionHandle;
use busbar_contract::abi::sdk::door::{abi_str, alias, statement};
use busbar_contract::abi::sdk::exchange::Op;
use busbar_contract::abi::sdk::hook::{
    statement_with_tail, tail, ConfigureCheckpoint, Decoded, Hook, HookOpen, Verdict,
};
use busbar_contract::plugin::markers::HookKind;
use std::sync::{Arc, Mutex};
use std::task::Poll;
const TAIL: &Tail = &tail(CLASS_GATE, PROMPT_RO, USER_NO);
const MARKS: &[MarkWord] = &[MarkWord {
    class: MARK_WORD_HOOK,
    _reserved: 0,
    word: abi_str(ALIAS),
}];
const REWRITES: &[Rewrite] = &[alias(ALIAS)];
busbar_contract::statement_extensions!(EXTENSIONS:[(busbar_contract::abi::mechanism::extensions::EXT_DECLARES,include_bytes!("../declares.json")),]);
const STATEMENT: Statement = Statement {
    settings_schema: busbar_contract::abi::mechanism::call::Blob::json(include_bytes!(
        "../settings.schema.json"
    )),
    extensions: EXTENSIONS,
    rewrites: REWRITES.as_ptr(),
    rewrites_len: REWRITES.len(),
    mark_words: MARKS.as_ptr(),
    mark_words_len: MARKS.len(),
    ..statement_with_tail(statement(NAME, env!("CARGO_PKG_VERSION"), 64), TAIL)
};
struct Complexity {
    config: Arc<Config>,
    checkpoint: Mutex<(u64, u64)>,
}
struct Invocation {
    config: Arc<Config>,
    payload: Result<Vec<u8>, &'static str>,
    answer: Option<Result<String, &'static str>>,
}
fn abstain(reason: &'static str) -> Poll<Verdict> {
    tracing::debug!(reason, "complexity abstained");
    Poll::Ready(Verdict::Abstain)
}
impl Hook for Complexity {
    fn decide(&self, view: &Decoded<'_>, op: &Op<'_, HookKind>) -> Poll<Verdict> {
        let Ok(invocation) = op.context(|| {
            Arc::new(Mutex::new(Invocation {
                config: self.config.clone(),
                payload: routing::payload(view, &self.config),
                answer: None,
            }))
        }) else {
            return abstain("invocation_context_unavailable");
        };
        let mut invocation = invocation.lock().unwrap_or_else(|e| e.into_inner());
        if invocation.answer.is_none() {
            let bytes = match &invocation.payload {
                Ok(bytes) => bytes,
                Err(reason) => return abstain(reason),
            };
            let Some(services) = op.host().and_then(|host| host.services()) else {
                return abstain("child_service_unavailable");
            };
            let mut output = vec![0; 65_536];
            // Same ticket, sequence and owned payload on every poll. Host memoization also
            // survives subsequent routing/failover invocations of this hook.
            let handle = CompletionHandle {
                ticket: op.ticket(),
                seq: 0,
                _reserved: 0,
            };
            let result = match services.hook_child(handle, bytes, &mut output) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok((200, bytes))) => routing::classify(bytes, &invocation.config),
                Poll::Ready(Ok(_)) => Err("classifier_unavailable"),
                Poll::Ready(Err(_)) => Err("child_refused_or_timed_out"),
            };
            invocation.answer = Some(result);
        }
        match invocation.answer.as_ref().expect("completed child") {
            Ok(label) => Poll::Ready(routing::verdict(label, view, &invocation.config)),
            Err(reason) => abstain(reason),
        }
    }
    fn configure(
        &self,
        settings: &serde_json::Map<String, serde_json::Value>,
        version: u64,
    ) -> Result<(), String> {
        self.checkpoint(settings, version).map(|_| ())
    }
    fn configure_checkpoint(
        &self,
        settings: &serde_json::Map<String, serde_json::Value>,
        version: u64,
    ) -> Result<Option<ConfigureCheckpoint>, String> {
        self.checkpoint(settings, version).map(Some)
    }
}
impl Complexity {
    fn checkpoint(
        &self,
        settings: &serde_json::Map<String, serde_json::Value>,
        version: u64,
    ) -> Result<ConfigureCheckpoint, String> {
        let proposed = Config::parse(&serde_json::Value::Object(settings.clone()).to_string())?;
        if proposed != *self.config {
            return Err(
                "complexity settings are immutable per instance; reload the hook configuration"
                    .into(),
            );
        }
        let bytes = serde_json::to_vec(self.config.as_ref()).map_err(|e| e.to_string())?;
        let mut current = self.checkpoint.lock().unwrap_or_else(|e| e.into_inner());
        let (revision, applied_version) = if version < current.1 {
            *current
        } else {
            (
                current
                    .0
                    .checked_add(1)
                    .ok_or("configuration revision exhausted")?,
                version,
            )
        };
        let checkpoint = ConfigureCheckpoint::new(revision, applied_version, bytes)?;
        *current = (revision, applied_version);
        Ok(checkpoint)
    }
}
/// Opens a hook with an immutable classification configuration.
pub struct Open;
impl HookOpen for Open {
    fn open(settings: &str) -> Result<Box<dyn Hook>, String> {
        Ok(Box::new(Complexity {
            config: Arc::new(Config::parse(settings)?),
            checkpoint: Mutex::new((0, 0)),
        }))
    }
}
busbar_contract::hook_door! {open:Open,statement:STATEMENT,}
