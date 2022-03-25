# simplep2pgossip
Simple CLI app for P2P gossiping written in Rust.

## Docs
```shell
$ cargo doc
```
## Building
For building with cargo, you'll need openssl lib. After that, just run:
```shell
$ cargo build [--release]
```

## Running tests
Tests require `mock_time` feature enabled: `$ cargo test --features mock_time`. 
But you can simply run:
```shell
$ ./test.sh
```

## Starting peer
Before starting a peer, you'll need to generate SSL certificate, for example:
```shell
$ openssl genrsa 4096 > key.pem
$ openssl req -x509 -days 1000 -new -key key.pem -out cert.pem
```

After that you can run your first node: 
```shell
$ RUST_LOG=simplep2pgossip=info,warp=info ./simplep2pgossip --cert=cert.pem --key=key.pem  --period=5 --port=8080 --bind=127.0.0.1
```

Then you can run other nodes, that will get a list of peers from the first peer with the key `--connect`:
```shell
$ RUST_LOG=simplep2pgossip=info,warp=info ./simplep2pgossip --cert=cert.pem --key=key.pem  --period=5 --port=8081 --bind=127.0.0.1 --connect=127.0.0.1:8080
```

All nodes are equal to each other, and share peer lists with each other, making it sustainable on case, 
when tracker is becoming unavailable.

## To improve
 * Proper trust model
 * Optimize lists replay