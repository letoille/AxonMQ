use std::collections::HashMap;

use super::super::protocol::publish;

#[derive(Clone)]
pub struct Store {
    backup_store: Vec<publish::Publish>,
    inflight_size: usize,
    inflight_store: HashMap<u16, (u64, Option<publish::Publish>)>,

    qos2_size: usize,
    qos2_recv_store: HashMap<u16, Option<publish::Publish>>,
}

impl Store {
    pub fn new(inflight_size: usize, qos2_size: usize) -> Self {
        Store {
            inflight_size: inflight_size,
            qos2_size: qos2_size,
            backup_store: Vec::new(),
            inflight_store: HashMap::new(),
            qos2_recv_store: HashMap::new(),
        }
    }

    pub fn extend(&mut self, other: Store) {
        self.backup_store.extend(other.backup_store);
        self.inflight_store.extend(other.inflight_store);
        self.qos2_recv_store.extend(other.qos2_recv_store);
    }

    pub fn inflight_size(&self) -> usize {
        self.inflight_store.len()
    }

    pub fn get_inflight_messages(
        &mut self,
        now: u64,
        resend_interval: u64,
    ) -> Vec<(u16, Option<publish::Publish>)> {
        let mut msgs = Vec::new();
        for (pkid, (tm, msg)) in self.inflight_store.iter_mut() {
            if *tm + resend_interval < now {
                if msgs.len() < self.inflight_size {
                    msgs.push((*pkid, msg.clone()));
                    *tm = now;
                } else {
                    break;
                }
            }
        }
        msgs
    }

    pub fn inflight_ack(&mut self, pkid: u16) {
        self.inflight_store.remove(&pkid).map(|(_, msg)| msg);
        if self.inflight_store.len() < self.inflight_size {
            if let Some(msg) = self.backup_store.pop() {
                self.inflight_store.insert(pkid, (0, Some(msg)));
            }
        }
    }

    pub fn inflight_rec(&mut self, pkid: u16) {
        let msg = self.inflight_store.get_mut(&pkid);
        if let Some((tm, m)) = msg {
            *m = None;
            *tm = coarsetime::Clock::now_since_epoch().as_secs();
        }
    }

    pub fn inflight_cmp(&mut self, pkid: u16) {
        self.inflight_store.remove(&pkid);
        if self.inflight_store.len() < self.inflight_size {
            if let Some(msg) = self.backup_store.pop() {
                self.inflight_store.insert(pkid, (0, Some(msg)));
            }
        }
    }

    pub fn inflight_insert(&mut self, mut msg: publish::Publish) -> bool {
        if self.inflight_store.len() < self.inflight_size {
            msg.dup = true;
            self.inflight_store.insert(
                msg.packet_id.unwrap_or(0),
                (coarsetime::Clock::now_since_epoch().as_secs(), Some(msg)),
            );
            true
        } else {
            self.backup_store.push(msg);
            false
        }
    }

    pub fn qos2_contains(&self, pkid: u16) -> bool {
        self.qos2_recv_store.contains_key(&pkid)
    }

    pub fn qos2_insert(&mut self, msg: publish::Publish) -> bool {
        if self.qos2_recv_store.contains_key(&msg.packet_id.unwrap()) {
            return true;
        }

        if self.qos2_recv_store.len() >= self.qos2_size {
            return false;
        }

        self.qos2_recv_store
            .insert(msg.packet_id.unwrap(), Some(msg));
        true
    }

    pub fn qos2_rel(&mut self, pkid: u16) -> Option<publish::Publish> {
        self.qos2_recv_store.remove(&pkid).and_then(|msg| msg)
    }
}
