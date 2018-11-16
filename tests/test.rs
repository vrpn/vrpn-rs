extern crate bytes;
extern crate mio;
extern crate socket2;
extern crate tokio;
extern crate vrpn;

use bytes::Bytes;
use tokio::prelude::*;
use vrpn::{
    async_io::{connect_tcp, ConnectionIp},
    LocalId, RemoteId, SenderId,
};

#[ignore] // because it requires an external server to be running.
#[test]
fn main() {
    let addr = "127.0.0.1:3883".parse().unwrap();
    let _conn = connect_tcp(addr)
        .and_then(|tcp_stream| Ok(ConnectionIp::new_client(None, None, tcp_stream)))
        .wait()
        .unwrap();
    println!("Hello, world!");
}
