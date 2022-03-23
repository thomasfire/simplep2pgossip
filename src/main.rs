extern crate simplep2pgossip;
use simplep2pgossip::server::run_server;
use simplep2pgossip::p2pcache::PeerCache;

use clap::Parser;
use env_logger::Env;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Send message every period seconds
    #[clap(long, default_value_t=1)]
    period: u32,
    /// Timeout for peer connection
    #[clap(long, default_value_t=30)]
    timeout: u32,
    /// IP to bind the server one
    #[clap(long, default_value="127.0.0.1")]
    bind: String,
    /// Port to bind on
    #[clap(long, default_value_t = 8080)]
    port: u16,
    /// Path to the TLS certificate
    #[clap(long, default_value="cert.pem")]
    cert: String,
    /// Path to the TLS private key
    #[clap(long, default_value="key.rsa")]
    key: String,
    /// address:port to make first connection. If absent, server will just listen to bound port
    #[clap(long)]
    connect: Option<String>
}

fn main() {
    let args: Args = Args::parse();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let cache = PeerCache::new(args.timeout);

    run_server(&args.bind, args.port, &args.cert, &args.key, &cache);
}
