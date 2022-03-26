use crate::p2pcache::{PeerCache, PeerList};

use reqwest;
use log::{error, info, trace};
use reqwest::StatusCode;
use rand::{distributions::Alphanumeric, Rng};
use serde_json::from_str as js_from_str;
use native_tls;

use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time;

pub type SignalT = Arc<(Mutex<bool>, Condvar)>;

fn random_msg() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

// Sends a message to all other peers.
fn messenger(name: &str, cache: &mut PeerCache) -> Result<bool, String> {
    let changed = Arc::new(AtomicBool::new(false));
    let mut handles = vec![];
    let all_peers: Vec<String> = cache.get_list()?.peers
        .iter()
        .filter(|x| x.address != name)
        .map(|x| x.address.clone())
        .collect();
    let message = random_msg();
    info!("Sending message `{}` to {:?}", message, all_peers);
    for peer in all_peers {
        let changed_copy = changed.clone();
        let changed_copy_or = changed.clone();
        let mut cache_copy = cache.clone();
        let mut cache_copy_or = cache.clone();
        let peer_copy = peer.clone();
        let peer_copy_or = peer.clone();
        let name_copy = name.to_string();
        let message_copy = message.clone();
        handles.push(thread::spawn( move || {
            let tls = native_tls::TlsConnector::builder()
                .use_sni(false)
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap();
            let client = reqwest::blocking::ClientBuilder::new()
                //.use_rustls_tls()
                .use_preconfigured_tls(tls)
                .timeout(time::Duration::new(10, 0))
                .build()
                .map_err(|err| {
                    error!("Error on building the client: {:?}", err);
                }).unwrap();
            client.get(format!("https://{}/message", peer_copy))
                .query(&[("peer_name", name_copy), ("msg", message_copy)])
                .send()
                .map(move |val| {
                    trace!("Message sent response: {:?}", val);
                    changed_copy.fetch_or(cache_copy.update_peer(&peer_copy, val.status() == StatusCode::OK).map_or(false, |x| x), Ordering::SeqCst);
                })
                .map_err(|err| {
                    info!("Couldn't send message to peer {}: {:?}", peer_copy_or, err);
                    changed_copy_or.fetch_or(cache_copy_or.update_peer(&peer_copy_or, false).unwrap_or(false), Ordering::SeqCst);
                }).unwrap_or(());
        }));
    }
    for thr in handles {
        thr.join().map_err(|err| {error!("Error on joning the thread: {:?}", err)}).unwrap_or(());
    }
    Ok(changed.load(Ordering::SeqCst))
}

// Sends current PeerList to all other peers.
fn updater(name: &str, cache: &mut PeerCache) -> Result<(), String> {
    let mut handles = vec![];
    let all_peers: Vec<String> = cache.get_list()?.peers
        .iter()
        .filter(|x| x.address != name)
        .map(|x| x.address.clone())
        .collect();
    trace!("PeerList: {:?}", &cache.get_list().unwrap_or(PeerList{peers: vec![]}));
    for peer in all_peers {
        let cache_copy = cache.clone();
        let peer_copy = peer.clone();
        let peer_copy_or = peer.clone();
        handles.push(thread::spawn(move || {
            let tls = native_tls::TlsConnector::builder()
                .use_sni(false)
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap();
            let client = reqwest::blocking::ClientBuilder::new()
                //.use_rustls_tls()
                .use_preconfigured_tls(tls)
                .timeout(time::Duration::new(10, 0))
                .build()
                .map_err(|err| {
                    error!("Error on building the client: {:?}", err);
                }).unwrap();
            client.get(format!("https://{}/update", peer_copy))
                .json(&cache_copy.get_list().unwrap_or(PeerList{peers: vec![]}))
                .send()
                .map(move |val| {
                    trace!("Update sent response: {:?}", val);
                    ()
                })
                .map_err(|err: reqwest::Error| {
                    info!("Couldn't send update to peer {}: {:?}", peer_copy_or, err);
                }).unwrap_or(());

        }));
    }
    for thr in handles {
        thr.join().map_err(|err| {error!("Error on joning the thread: {:?}", err)}).unwrap_or(());
    }
    Ok(())
}
// Retrieves initial PeerList from another peer.
fn connect_to_first_peer(self_name: &str, cache: &mut PeerCache, address: &str) -> Result<(), String> {
    let self_name_copy = self_name.to_string();
    let tls = native_tls::TlsConnector::builder()
        .use_sni(false)
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let client = reqwest::blocking::ClientBuilder::new()
        .use_preconfigured_tls(tls)// TODO delete on prod
        .timeout(time::Duration::new(10, 0))
        .build()
        .map_err(|err| {
            error!("Error on building the client: {:?}", err);
            format!("Error on building the client: {:?}", err)
        })?;
    client.get(format!("https://{}/peers/{}", address, self_name_copy))
        .body(self_name_copy)
        .send()
        .map_or_else(|err| {
            info!("Couldn't request peers from `{}`: {:?}", address, err);
            Err("Couldn't connect".to_string())
        }, |val: reqwest::blocking::Response| {
            trace!("{:?}", val);
            if val.status() == StatusCode::OK {
                cache.update_from_list(
                    &js_from_str(
                        match val.text() {
                            Ok(val) => val,
                            Err(err) => return Err(format!("{:?}", err))
                        }.as_str()
                        // .await
                    ).map_err(|err: serde_json::Error| -> String { format!("{:?}", err) })?)
                .map_err(|err| { format!("{:?}", err)})?;
            }
            Ok(())
        })
}

/// Run services:
///  * retrieve first PeerList of *connect* is available
///  * send random message every *period* to all other peers
///  * keep track on updates and send them to all other peers
///  * clean up old peers every timeout/2 seconds
pub fn run_saabisu(self_name: &str, connect: &Option<String>, period: u32, timeout: u32, cache: &PeerCache) {
    let mut cache_copy = cache.clone();
    let mut cache_copy_msg = cache.clone();
    let mut cache_copy_upd = cache.clone();
    let mut cache_copy_clear = cache.clone();
    let name_copy = self_name.to_string();
    let name_copy_msg = self_name.to_string();
    let name_copy_upd = self_name.to_string();

    connect.clone().map(|first_peer| {
        thread::spawn(move || {
            match connect_to_first_peer(&name_copy, &mut cache_copy, &first_peer) {
                Ok(_) => info!("Connected to `{}`", first_peer),
                Err(err) => error!("Error on connecting to `{}`: `{}`", first_peer, err)
            };
        });
    });
    thread::spawn(move || {
        loop {
            thread::sleep(time::Duration::new(period as u64, 0));
            trace!("Sending messages");
            match messenger(&name_copy_msg, &mut cache_copy_msg) {
                Ok(updated) => if updated { cache_copy_msg.signaler.broadcast().map_err(|_| {error!("Error on broadcasting");}).unwrap_or(()); },
                Err(err) => error!("Error on sending messages: {:?}", err)
            };
        }
    });
    thread::spawn(move || {
        loop {
            cache_copy_upd.signaler.wait().map_err(|_| {error!("Error on waiting");}).unwrap_or(());
            trace!("Replaying updates");
            updater(&name_copy_upd, &mut cache_copy_upd).map_err(|err| {
                error!("Error on sending updates: {}", err);
            }).unwrap_or(());
        }
    });
    thread::spawn(move || {
       loop {
           thread::sleep(time::Duration::new((timeout as u64) / 2, 0));
           cache_copy_clear.cleanup_old_peers().map_err(|err| {
               error!("Error on sending updates: {}", err);
           }).unwrap_or(());
       }
    });
}