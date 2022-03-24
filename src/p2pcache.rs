use crate::waiter::Waiter;

use serde::{Deserialize, Serialize};
use log::{error, trace};
use chrono::prelude::*;

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

const MS_IN_SEC: u32 = 1000;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerState {
    pub path: String,
    pub timestamp: i64,
    pub available: bool
}

impl PartialEq for PeerState {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.available == self.available && self.timestamp == other.timestamp
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PeerList {
    pub peers: Vec<PeerState>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PeerMap {
    pub peers: BTreeMap<String, PeerState>
}

#[derive(Debug, Clone)]
pub struct PeerCache {
    peers: Arc<RwLock<PeerMap>>,
    timeout: u32,
    pub signaler: Waiter,
    #[cfg(feature = "mock_time")]
    pub current_time: i64
}


impl PeerCache {
    #[cfg(not(feature = "mock_time"))]
    pub fn new(timeout: u32) -> Self {
        PeerCache { peers: Arc::new(RwLock::new(PeerMap { peers: BTreeMap::new() })), timeout: timeout*MS_IN_SEC, signaler: Waiter::new(), }
    }
    #[cfg(feature = "mock_time")]
    pub fn new(timeout: u32) -> Self {
        PeerCache { peers: Arc::new(RwLock::new(PeerMap { peers: BTreeMap::new() })),
            timeout: timeout*MS_IN_SEC,
            signaler: Waiter::new(),
            current_time: 0,
        }
    }

    #[cfg(not(feature = "mock_time"))]
    fn timestamp_now(&self) -> i64 {
        Utc::now().timestamp_millis()
    }
    #[cfg(feature = "mock_time")]
    fn timestamp_now(&self) -> i64 {
        self.current_time
    }

    #[cfg(feature = "mock_time")]
    pub fn set_current_time(&mut self, time: i64) {
        self.current_time = time;
    }

    pub fn cleanup_old_peers(&mut self) -> Result<(), String> {
        let current_utc = self.timestamp_now();
        self.peers.write().map(|mut cache| {
            for peer in &cache.peers.values().map(|val| val.clone()).collect::<Vec<PeerState>>() {
                if !peer.available && current_utc - peer.timestamp > self.timeout as i64 {
                    cache.peers.remove(&peer.path);
                }
            }
        }).map_err(|err| { error!("Poison error: {:?}", err); format!("Poison error: {:?}", err) })?;
        Ok(())
    }

    pub fn update_peer(&mut self, path: &str, available: bool) -> Result<bool, String> {
        let mut changed = false;
        self.peers.write().map(|mut cache| {
            match cache.peers.get(path) {
                Some(val) => {
                    if val.available != available || available {
                        changed = val.available != available;
                    }
                },
                None => {
                    changed = true;
                }
            };
            if changed || available {
                cache.peers.insert(path.to_string(), PeerState { path: path.to_string(), available, timestamp: self.timestamp_now() });
            }
        }).map_err(|err| { error!("Poison error: {:?}", err); format!("Poison error: {:?}", err) })?;
        Ok(changed)
    }

    pub fn update_from_list(&mut self, other: &PeerList) -> Result<bool, String> {
        let mut changed = false;
        self.peers.write().map(|mut cache| {
            for peer in &other.peers {
                match cache.peers.get(&peer.path) {
                    Some(val) => {
                        if val.timestamp > peer.timestamp { // skip old data
                            continue;
                        }
                        changed = val.available != peer.available;
                    },
                    None => {
                        changed = true;
                    }
                };
                cache.peers.insert(peer.path.clone(), peer.clone()); // update in any case
            }
        }).map_err(|err| { error!("Poison error: {:?}", err); format!("Poison error: {:?}", err) })?;
        Ok(changed)
    }

    pub fn get_list(&self) -> Result<PeerList, String> {
        let mut changed = false;
        self.peers.read().map(|cache| {
            Ok(PeerList{peers: cache.peers.values().map(|val| val.clone()).collect::<Vec<PeerState>>()})
        }).map_err(|err| { error!("Poison error: {:?}", err); format!("Poison error: {:?}", err) })?
    }

}