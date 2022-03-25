use crate::p2pcache::{PeerCache, PeerList};

use warp::{http::{StatusCode, Response}, Filter};
use serde_json::to_string as js_to_string;
use log::{error, info, warn};

use std::net::SocketAddr;
use std::collections::HashMap;

#[tokio::main]
pub async fn run_server(bind: &str, port: u16, cert: &str, key: &str, cache: &PeerCache) {
    let cache_clone = cache.clone();
    let cache_clone_mut = cache.clone();

    // Receives a self-name of the peer and returns a list of peers. Also adds peer to the list.
    let peers_srv = warp::path("peers")
        .and(warp::get())
        .and(warp::path::param())
        .map(move |back_name: String|  {
            let mut mut_cache = cache_clone.clone();
            mut_cache.update_peer(&back_name, true).map_or_else(|err| {
                error!("Error on updating the peer the PeerList: {}", err);
                Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body("".to_string())
            }, |val: bool| -> Result<Response<std::string::String>, warp::http::Error> {
                if val { mut_cache.signaler.broadcast().map_err(|_| {error!("Error on broadcasting");}).unwrap_or(()); }
                cache_clone.get_list().map_or_else(|err| {
                    error!("Error on getting the PeerList: {}", err);
                    Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body("".to_string())
                },|peers_l: PeerList| {
                    js_to_string(&peers_l).map_or_else(|err| {
                        error!("Error on jsoning the PeerList: {:?}", err);
                        Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body("".to_string())
                    }, |v| { Response::builder().body(v) })

                })
            })
        });

    // Handle for receiving PeerLists from others. On update, broadcasts for waiters.
    let update_peers_srv = warp::get()
        .and(warp::path("update"))
        .and(warp::body::json::<PeerList>())
        .map(move |new_peers_list: PeerList| {
            let updated = match cache_clone_mut.clone().update_from_list(&new_peers_list) {
                Ok(val) => val,
                Err(err) => {
                    error!("Error on updating the PeerList: {}", err);
                    return Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body("".to_string());
                }
            };
            if updated { cache_clone_mut.signaler.broadcast().map_err(|_| {error!("Error on broadcasting");}).unwrap_or(()); }
            Response::builder().status(StatusCode::OK).body("".to_string())
        });

    // Handle for receiving the messages. Actually, doesn't update state of peers.
    let message_srv = warp::get()
        .and(warp::path("message"))
        .and(warp::query::<HashMap<String, String>>())
        .map(move |simple_map: HashMap<String, String>| {
            let peer_name = match simple_map.get("peer_name") {
                Some(val) => val,
                None => {
                    warn!("No parameter `peer_name`");
                    return StatusCode::BAD_REQUEST;
                }
            };
            let msg = match simple_map.get("msg") {
                Some(val) => val,
                None => {
                    warn!("No parameter `msg`");
                    return StatusCode::BAD_REQUEST;
                }
            };
            info!("Received message `{}` from `{}` ", msg, peer_name);
            StatusCode::OK
        });

    let any_srv = warp::any().map(|| {
        warn!("Default path");
        StatusCode::BAD_REQUEST
    });

    let routes = warp::get().and(
        peers_srv
            .or(update_peers_srv)
            .or(message_srv)
            .or(any_srv),
    );

    warp::serve(routes)
        .tls()
        .cert_path(cert)
        .key_path(key)
        .run(format!("{}:{}", bind, port).parse::<SocketAddr>().unwrap())
        .await;
}