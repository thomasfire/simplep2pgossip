use crate::p2pcache::{PeerCache, PeerList};

use reqwest;
use log::{error, info};
use reqwest::{Response, StatusCode};
use rand::{distributions::Alphanumeric, Rng};
use serde_json::{from_str as js_from_str, to_string as js_to_string};

use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time;
use serde::Serialize;
use warp::hyper::body::HttpBody;

pub type SignalT = Arc<(Mutex<bool>, Condvar)>;

fn random_msg() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

async fn messenger(name: &str, cache: &mut PeerCache) -> Result<bool, String> {
    let mut changed = Arc::new(AtomicBool::new(false));
    for peer in cache.get_list()?.peers {
        let mut changed_copy = changed.clone();
        let mut changed_copy_or = changed.clone();
        let mut cache_copy = cache.clone();
        let mut cache_copy_or = cache.clone();
        let peer_copy = peer.clone();
        let peer_copy_or = peer.clone();
        let name_copy = name.to_string();
        tokio::spawn(async move {
            let client = reqwest::ClientBuilder::new()
                .use_rustls_tls()
                .danger_accept_invalid_certs(true) // TODO delete on prod
                .timeout(time::Duration::new(10, 0))
                .build()
                .map_err(|err| {
                    error!("Error on building the client: {:?}", err);
                    format!("Error on building the client: {:?}", err)
                })?;
            client.post(format!("https://{}/message", peer_copy.path))
                .form(&[("peer_name", name_copy), ("msg", random_msg())])
                .send()
                .await
                .and_then(move |val| {
                    changed_copy.fetch_or(cache_copy.update_peer(&peer_copy.path, val.status() == StatusCode::OK).map_or(false, |x| x), Ordering::SeqCst);
                    Ok(())
                })
                .or_else(|err: reqwest::Error| -> Result<(), String> {
                    info!("Couldn't send message to peer {}: {:?}", peer_copy_or.path, err);
                    changed_copy_or.fetch_or(cache_copy_or.update_peer(&peer_copy_or.path, false)?, Ordering::SeqCst);
                    Ok(())
                })
        });
    }
    Ok(changed.load(Ordering::SeqCst))
}

async fn updater(cache: &mut PeerCache) -> Result<(), String> {
    for peer in cache.get_list()?.peers {
        let mut cache_copy = cache.clone();
        let mut cache_copy_or = cache.clone();
        let peer_copy = peer.clone();
        let peer_copy_or = peer.clone();
        tokio::spawn(async move {
            let client = reqwest::ClientBuilder::new()
                .use_rustls_tls()
                .danger_accept_invalid_certs(true) // TODO delete on prod
                .timeout(time::Duration::new(10, 0))
                .build()
                .map_err(|err| {
                    error!("Error on building the client: {:?}", err);
                    format!("Error on building the client: {:?}", err)
                })?;
            client.post(format!("https://{}/update", peer_copy.path))
                .body(js_to_string(&cache_copy.get_list()?).map_err(|err| {
                    error!("Failed to serialize data: {:?}", err);
                    format!("Failed to serialize data: {:?}", err)
                })?)
                .send()
                .await
                .and_then(move |_val| {
                    Ok(())
                })
                .or_else(|err: reqwest::Error| -> Result<(), String> {
                    info!("Couldn't send update to peer {}: {:?}", peer_copy_or.path, err);
                    Ok(())
                })
        });
    }
    Ok(())
}

fn connect_to_first_peer(name: &str, cache: &mut PeerCache, path: &str) -> Result<(), String> {
    let name_copy = name.to_string();
    let client = reqwest::blocking::ClientBuilder::new()
        .use_rustls_tls()
        .danger_accept_invalid_certs(true) // TODO delete on prod
        .timeout(time::Duration::new(10, 0))
        .build()
        .map_err(|err| {
            error!("Error on building the client: {:?}", err);
            format!("Error on building the client: {:?}", err)
        })?;
    client.get(format!("https://{}/peers", path))
        .body(name_copy)
        .send()
        .map_or_else(|err| {
            info!("Couldn't send update to peer {}: {:?}", path, err);
            Ok(())
        }, |val: reqwest::blocking::Response| {
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

pub fn run_saabisu(name: &str, connect: &Option<String>, period: u32, timeout: u32, cache: &PeerCache) {
    let mut cache_copy = cache.clone();
    let mut cache_copy_msg = cache.clone();
    let mut cache_copy_upd = cache.clone();
    let name_copy = name.to_string();
    let name_copy_msg = name.to_string();
    connect.clone().map(|first_peer| {
        thread::spawn(move || {
            connect_to_first_peer(&name_copy, &mut cache_copy, &first_peer);
        });
    });
    let _ = thread::spawn(move || {
        loop {
            thread::sleep(time::Duration::new(period as u64, 0));
            messenger(&name_copy_msg, &mut cache_copy_msg);
        }
    });
    let _ = thread::spawn(move || {
        loop {
            cache_copy_upd.signaler.wait();
            updater(&mut cache_copy_upd);
        }
    });
}