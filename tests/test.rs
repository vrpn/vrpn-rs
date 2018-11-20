extern crate bytes;
extern crate mio;
extern crate socket2;
extern crate tokio;
extern crate vrpn;

use bytes::Bytes;
use tokio::prelude::*;
use vrpn::{async_io::ConnectionIp, LocalId, RemoteId, SenderId, ServerInfo};

#[ignore] // because it requires an external server to be running.
#[test]
fn main() {
    // let server = "127.0.0.1:3883".parse::<ServerInfo>().unwrap();
    // let conn = ConnectionIp::new_client(server, None, None).unwrap();
    // let _conn = connect_tcp(addr)
    //     .and_then(|tcp_stream| Ok(ConnectionIp::new_client(None, None, tcp_stream)))
    //     .wait()
    //     .unwrap();
    println!("Hello, world!");
}
