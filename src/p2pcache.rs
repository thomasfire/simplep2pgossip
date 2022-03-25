use crate::waiter::Waiter;

use log::{error};
#[cfg(not(feature = "mock_time"))]
use chrono::prelude::*;

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

const MS_IN_SEC: u32 = 1000;

/// Represents a peer - it has address, timestamp of last request and availability,
/// which shows last connection result.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerState {
    pub address: String,
    pub timestamp: i64,
    pub available: bool
}

impl PartialEq for PeerState {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address && self.available == self.available && self.timestamp == other.timestamp
    }
}

/// Represents a list of peers, mostly needed to be sent to others.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PeerList {
    pub peers: Vec<PeerState>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PeerMap {
    pub peers: BTreeMap<String, PeerState>
}

/// PeerCache stores lists of peers with it's states and timestamps,
/// and manages updates. Also contains signaler, which can be used
/// for waiting/signaling purposes externally.
/// For testing purposes, has feature `mock_time`, which make it possible
/// to manage timestamps within the tests.
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

    /// Removes peers, that couldn't be connected for `timeout` seconds.
    pub fn cleanup_old_peers(&mut self) -> Result<(), String> {
        let current_utc = self.timestamp_now();
        self.peers.write().map(|mut cache| {
            for peer in &cache.peers.values().map(|val| val.clone()).collect::<Vec<PeerState>>() {
                if !peer.available && current_utc - peer.timestamp > self.timeout as i64 {
                    cache.peers.remove(&peer.address);
                }
            }
        }).map_err(|err| { error!("Poison error: {:?}", err); format!("Poison error: {:?}", err) })?;
        Ok(())
    }

    /// Inserts a new peer or updates state for existing one
    /// and returns Result with bool, which means:
    ///  * true - peer list has changed and need to be sent to other peers
    ///  * false - peer remains the same and no resync is required
    /// List is considered to be changed when either a new item has been inserted
    /// or state of the existing one has been changed. Also, in any case, updates timestamp.
    pub fn update_peer(&mut self, address: &str, available: bool) -> Result<bool, String> {
        let mut changed = false;
        self.peers.write().map(|mut cache| {
            match cache.peers.get(address) {
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
                cache.peers.insert(address.to_string(), PeerState { address: address.to_string(), available, timestamp: self.timestamp_now() });
            }
        }).map_err(|err| { error!("Poison error: {:?}", err); format!("Poison error: {:?}", err) })?;
        Ok(changed)
    }

    /// Updates current list from incoming PeerList
    /// and returns Result with bool, which means:
    ///  * true - peer list has changed and need to be sent to other peers
    ///  * false - peer remains the same and no resync is required
    /// List is considered to be changed when either a new items have been inserted
    /// or state of the existing ones have been changed.
    /// For this method, timestamps does matter as the only newer entries are considered.
    pub fn update_from_list(&mut self, other: &PeerList) -> Result<bool, String> {
        let mut changed = false;
        self.peers.write().map(|mut cache| {
            for peer in &other.peers {
                match cache.peers.get(&peer.address) {
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
                cache.peers.insert(peer.address.clone(), peer.clone()); // update in any case
            }
        }).map_err(|err| { error!("Poison error: {:?}", err); format!("Poison error: {:?}", err) })?;
        Ok(changed)
    }

    /// Returns PeerList of peers.
    pub fn get_list(&self) -> Result<PeerList, String> {
        self.peers.read().map(|cache| {
            Ok(PeerList{peers: cache.peers.values().map(|val| val.clone()).collect::<Vec<PeerState>>()})
        }).map_err(|err| { error!("Poison error: {:?}", err); format!("Poison error: {:?}", err) })?
    }

}