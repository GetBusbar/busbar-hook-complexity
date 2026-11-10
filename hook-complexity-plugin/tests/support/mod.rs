// SPDX-License-Identifier: Apache-2.0
//! No general host services are granted to the classifier fixture. Child authority is separate.
use busbar_contract::services::*;
pub struct NoGeneralServices;
#[allow(unused_variables)]
impl HostServices for NoGeneralServices {
    fn now(&self) -> Reading {
        panic!("classification hook called an ungranted host service")
    }
    fn dest_judge(&self, dest: &str, class: u32, flags: u32, later: Option<Later>) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn records_get(&self, caller: &Caller, kind: &str, key: &[u8], later: Later) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn records_list(&self, caller: &Caller, list: RecordsList, later: Later) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn records_claim(
        &self,
        caller: &Caller,
        kind: &str,
        key: &[u8],
        ttl_ms: u64,
        later: Later,
    ) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn sign(&self, caller: &Caller, data: &[u8]) -> Stored {
        panic!("classification hook called an ungranted host service")
    }
    fn trust_sight(&self, caller: &Caller, counterparty: &str, hash: &str, later: Later) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn trust_unreached(&self, caller: &Caller, counterparty: &str) -> Stored {
        panic!("classification hook called an ungranted host service")
    }
    fn trust_sight_item(
        &self,
        caller: &Caller,
        counterparty: &str,
        item: &str,
        digest: &str,
        later: Later,
    ) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn trust_decide(
        &self,
        caller: &Caller,
        _ticket: busbar_contract::abi::mechanism::ticket::Ticket,
        key: TrustKeyRef<'_>,
        expected: Option<&str>,
        approve: bool,
        later: Later,
    ) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn trust_state(&self, caller: &Caller, counterparty: &str) -> Stored {
        panic!("classification hook called an ungranted host service")
    }
    fn trust_serves(
        &self,
        caller: &Caller,
        counterparty: &str,
        item: Option<&str>,
        digest: Option<&str>,
    ) -> Stored {
        panic!("classification hook called an ungranted host service")
    }
    fn trust_due(&self, caller: &Caller) -> Stored {
        panic!("classification hook called an ungranted host service")
    }
    fn trust_verify(
        &self,
        caller: &Caller,
        counterparty: &str,
        payload: &[u8],
        signatures: &[u8],
    ) -> Stored {
        panic!("classification hook called an ungranted host service")
    }
    fn entitlement_check(&self, caller: &Caller, unit: Option<u64>, target: &str) -> Stored {
        panic!("classification hook called an ungranted host service")
    }
    fn session_emit(&self, caller: &Caller, session: u64, bytes: &[u8]) -> Stored {
        panic!("classification hook called an ungranted host service")
    }
    fn ticket_mint(
        &self,
        caller: &Caller,
        unit: Option<u64>,
        ttl_secs: u64,
        record: &[u8],
        later: Later,
    ) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn ticket_take(&self, caller: &Caller, unit: Option<u64>) -> Stored {
        panic!("classification hook called an ungranted host service")
    }
    fn random_fill(&self, len: u64) -> Stored {
        panic!("classification hook called an ungranted host service")
    }
    fn records_secret(&self, kind: &str, id: &str, later: Later) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn unit_nest(&self, caller: &Caller, unit: Option<u64>, ask: NestAsk, later: Later) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn work_open(
        &self,
        caller: &Caller,
        unit: Option<u64>,
        kind: &str,
        record: &[u8],
        later: Later,
    ) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn work_find(&self, caller: &Caller, unit: Option<u64>, reference: &[u8], later: Later) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn work_settle(
        &self,
        caller: &Caller,
        unit: Option<u64>,
        handle: u64,
        record: &[u8],
        later: Later,
    ) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn work_resume(&self, caller: &Caller, unit: Option<u64>, handle: u64, later: Later) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn disk_append(&self, dest: &DiskDest, bytes: Vec<u8>, later: Later) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn verify_lookup(&self, caller: &Caller, key: &[u8], later: Later) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn verify_store(&self, caller: &Caller, key: &[u8], entry: &[u8], ttl_ms: u64) -> Stored {
        panic!("classification hook called an ungranted host service")
    }
    fn content_scan(
        &self,
        caller: &Caller,
        unit: Option<u64>,
        content: &[u8],
        later: Later,
    ) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn hook_call(&self, caller: &Caller, unit: Option<u64>, ask: HookAsk, later: Later) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
    fn snapshot_read(&self, caller: &Caller, scope: u32, later: Later) -> Ran {
        panic!("classification hook called an ungranted host service")
    }
}
