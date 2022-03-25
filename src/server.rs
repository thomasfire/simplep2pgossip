use crate::p2pcache::{PeerCache, PeerList};

use warp::{http::{StatusCode, Response}, Filter};
use warp::ws::{Message, WebSocket};
use serde_json::{from_str as js_from_str, to_string as js_to_string};
use log::{error, info, warn};

use std::net::SocketAddr;
use std::collections::{BTreeMap, HashMap};

#[tokio::main]
pub async fn run_server(bind: &str, port: u16, cert: &str, key: &str, cache: &PeerCache) {
    let cache_clone = cache.clone();
    let mut cache_clone_mut = cache.clone();
    let peers_srv = warp::path("peers")
        .and(warp::get())
        .and(warp::path::param())
        .map(move |peer_name: String|  {
            let mut mut_cache = cache_clone.clone();
            mut_cache.update_peer(&peer_name, true).map_or_else(|err| {
                error!("Error on updating the peer the PeerList: {}", err);
                Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body("".to_string())
            }, |val: bool| -> Result<Response<std::string::String>, warp::http::Error> {
                cache_clone.get_list().map_or_else(|err| {
                    error!("Error on getting the PeerList: {}", err);
                    Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body("".to_string())
                },|peers_l: PeerList| {
                    js_to_string(&peers_l).map_or_else(|err| {
                        error!("Error on jsoning the PeerList: {:?}", err);
                        Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body("".to_string())
                    }, |v| { Response::builder().body(v) })

                })
            }).map(|val| {mut_cache.signaler.broadcast(); val})
        });

    let update_peers_srv = warp::get()
        .and(warp::path("update"))
        .and(warp::body::json::<PeerList>())
        .map(move |new_data: PeerList| {
            let updated = match cache_clone_mut.clone().update_from_list(&new_data) {
                Ok(val) => val,
                Err(err) => {
                    error!("Error on updating the PeerList: {}", err);
                    return Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body("".to_string());
                }
            };
            if (updated) { cache_clone_mut.signaler.broadcast(); }
            Response::builder().status(StatusCode::OK).body("".to_string())
        });

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
            info!("Received message `{}` from `{}` ", peer_name, msg);
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