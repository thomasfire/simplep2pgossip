use crate::p2pcache::{PeerCache, PeerList};

use reqwest;
use log::{error, info, trace};
use reqwest::{Response, StatusCode};
use rand::{distributions::Alphanumeric, Rng};
use serde_json::{from_str as js_from_str, json, to_string as js_to_string};
use native_tls;
use serde::Serialize;
use futures;
use warp::hyper::body::HttpBody;

use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time;
use warp::trace;

pub type SignalT = Arc<(Mutex<bool>, Condvar)>;

fn random_msg() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

fn messenger(name: &str, cache: &mut PeerCache) -> Result<bool, String> {
    let mut changed = Arc::new(AtomicBool::new(false));
    let mut handles = vec![];
   // trace!("{:?}", cache.get_list()?.peers);
    for peer in cache.get_list()?.peers {
        if peer.path == name {
            continue;
        }
        //trace!("Peer: {:?}", peer);
        let mut changed_copy = changed.clone();
        let mut changed_copy_or = changed.clone();
        let mut cache_copy = cache.clone();
        let mut cache_copy_or = cache.clone();
        let peer_copy = peer.clone();
        let peer_copy_or = peer.clone();
        let name_copy = name.to_string();
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
            client.get(format!("https://{}/message", peer_copy.path))
                .query(&[("peer_name", name_copy), ("msg", random_msg())])
                .send()
                .map(move |val| {
                    trace!("Message sent response: {:?}", val);
                    changed_copy.fetch_or(cache_copy.update_peer(&peer_copy.path, val.status() == StatusCode::OK).map_or(false, |x| x), Ordering::SeqCst);
                })
                .map_err(|err| {
                    info!("Couldn't send message to peer {}: {:?}", peer_copy_or.path, err);
                    changed_copy_or.fetch_or(cache_copy_or.update_peer(&peer_copy_or.path, false).unwrap_or(false), Ordering::SeqCst);
                });
        }));
    }
    for thr in handles {
        thr.join();
    }
    //futures::future::join_all(handles).await;
    Ok(changed.load(Ordering::SeqCst))
}

fn updater(name: &str, cache: &mut PeerCache) -> Result<(), String> {
    let mut handles = vec![];
    for peer in cache.get_list()?.peers {
        if peer.path == name {
            continue;
        }
       // trace!("Peer: {:?}", peer);
        let mut cache_copy = cache.clone();
        let mut cache_copy_or = cache.clone();
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
            client.get(format!("https://{}/update", peer_copy.path))
                .json(&cache_copy.get_list().unwrap_or(PeerList{peers: vec![]}))
                .send()
                .map(move |val| {
                    trace!("Update sent response: {:?}", val);
                    ()
                })
                .map_err(|err: reqwest::Error| {
                    info!("Couldn't send update to peer {}: {:?}", peer_copy_or.path, err);
                });

        }));
    }
    for thr in handles {
        thr.join();
    }
    //futures::future::join_all(handles).await;
    Ok(())
}

fn connect_to_first_peer(name: &str, cache: &mut PeerCache, path: &str) -> Result<(), String> {
    let name_copy = name.to_string();
    let tls = native_tls::TlsConnector::builder()
        .use_sni(false)
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let client = reqwest::blocking::ClientBuilder::new()
        //.use_rustls_tls()
        .use_preconfigured_tls(tls)// TODO delete on prod
        .timeout(time::Duration::new(10, 0))
        .build()
        .map_err(|err| {
            error!("Error on building the client: {:?}", err);
            format!("Error on building the client: {:?}", err)
        })?;
    client.get(format!("https://{}/peers/{}", path, name_copy))
        .body(name_copy)
        .send()
        .map_or_else(|err| {
            info!("Couldn't send update to peer {}: {:?}", path, err);
            Ok(())
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

pub fn run_saabisu(name: &str, connect: &Option<String>, period: u32, timeout: u32, cache: &PeerCache) {
    let mut cache_copy = cache.clone();
    let mut cache_copy_msg = cache.clone();
    let mut cache_copy_upd = cache.clone();
    let name_copy = name.to_string();
    let name_copy_msg = name.to_string();
    let name_copy_upd = name.to_string();
    connect.clone().map(|first_peer| {
        thread::spawn(move || {
            connect_to_first_peer(&name_copy, &mut cache_copy, &first_peer);
            info!("Connected to `{}`", first_peer);
        });
    });
    thread::spawn(move || {
        loop {
            thread::sleep(time::Duration::new(period as u64, 0));
            info!("Sending messages");
            match messenger(&name_copy_msg, &mut cache_copy_msg) {
                Ok(updated) => if updated { cache_copy_msg.signaler.broadcast(); },
                Err(err) => error!("Error on sending messages: {:?}", err)
            };
        }
    });
    thread::spawn(move || {
        loop {
            cache_copy_upd.signaler.wait();
            info!("Replaying updates");
            updater(&name_copy_upd, &mut cache_copy_upd);
        }
    });
}