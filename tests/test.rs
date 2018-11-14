extern crate bytes;
extern crate mio;
extern crate socket2;
extern crate tokio;
extern crate vrpn;

use bytes::Bytes;
use tokio::prelude::*;
use vrpn::{
    vrpn_tokio::{connect_tcp, ConnectionIp},
    LocalId, RemoteId, SenderId,
};

#[test]
fn main() {
    let addr = "127.0.0.1:3883".parse().unwrap();
    let _conn = connect_tcp(addr)
        .and_then(|tcp_stream| Ok(ConnectionIp::new_client(None, None, tcp_stream)))
        .wait()
        .unwrap();
    println!("Hello, world!");
}
