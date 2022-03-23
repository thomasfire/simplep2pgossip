use std::fmt::format;
use crate::p2pcache::{PeerCache, PeerList};

use reqwest;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time;
use log::{error, info};
use reqwest::StatusCode;



async fn updater(name: &str, cache: &mut PeerCache) -> Result<bool, String> {
    let mut changed = Arc::new(AtomicBool::new(false));
    for peer in cache.get_list()?.peers {
        let mut changed_copy = changed.clone();
        let mut changed_copy_or = changed.clone();
        let mut cache_copy = cache.clone();
        let mut cache_copy_or = cache.clone();
        let peer_copy = peer.clone();
        let peer_copy_or = peer.clone();
        tokio::spawn(async move {
            let client = reqwest::ClientBuilder::new()
                .use_rustls_tls()
                .danger_accept_invalid_certs(true) // TODO delete on prod
                .timeout(time::Duration::new(30, 0))
                .build()
                .map_err(|err| {
                    error!("Error on building the client: {:?}", err);
                    format!("Error on building the client: {:?}", err)
                })?;
            client.post(format!("https://{}/message", peer_copy.path))
                .body("the exact body that is sent")
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
                //.await
        });
    }
    Ok(changed.load(Ordering::SeqCst))
}

pub fn run_saabisu(name: &str, connect: &Option<String>, period: u32, timeout: u32, cache: &PeerCache) {

}