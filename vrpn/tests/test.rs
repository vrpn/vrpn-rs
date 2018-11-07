extern crate bytes;
extern crate mio;
extern crate socket2;
extern crate tokio;
extern crate vrpn;

use bytes::Bytes;
use tokio::prelude::*;
use vrpn::{
    base::types::{LocalId, RemoteId, SenderId},
    connect::connect_tcp,
    connection::translationtable::TranslationTable,
    ConnectionIP,
};

#[test]
fn main() {
    let addr = "127.0.0.1:3883".parse().unwrap();
    let mut table: TranslationTable<SenderId> = TranslationTable::new();
    table
        .add_remote_entry(
            Bytes::from_static(b"asdf"),
            RemoteId(SenderId(0)),
            LocalId(SenderId(0)),
        )
        .expect("Failed adding remote entry");
    let _conn = connect_tcp(addr)
        .and_then(|tcp_stream| Ok(ConnectionIP::new_client(None, None, tcp_stream)))
        .wait()
        .unwrap();
    println!("Hello, world!");
}
