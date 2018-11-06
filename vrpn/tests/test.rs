extern crate bytes;
extern crate mio;
extern crate socket2;
extern crate tokio;
extern crate vrpn;

use bytes::{Bytes, BytesMut};
use tokio::{io, net::TcpStream, prelude::*};
use vrpn::{
    base::{
        constants,
        cookie::{self, check_ver_nonfile_compatible, CookieData},
        types::{LocalId, RemoteId, SenderId},
    },
    buffer::{buffer, unbuffer, Buffer, BufferSize, ConstantBufferSize, Output, Unbuffer},
    connection::translationtable::TranslationTable,
    prelude::*,
    vrpn_tokio::{connect_tcp, make_tcp_socket},
    ConnectionIP,
};

#[test]
fn sync_connect() {
    let addr = "127.0.0.1:3883".parse().unwrap();

    let sock = make_tcp_socket(addr).expect("failure making the socket");
    let stream = TcpStream::connect_std(sock, &addr, &tokio::reactor::Handle::default())
        .wait()
        .unwrap();

    let cookie = CookieData::from(constants::MAGIC_DATA);
    let mut send_buf = BytesMut::with_capacity(cookie.buffer_size());
    cookie.buffer_ref(&mut send_buf).unwrap();
    let (stream, _) = io::write_all(stream, send_buf.freeze()).wait().unwrap();

    let (_stream, read_buf) = io::read_exact(stream, vec![0u8; CookieData::constant_buffer_size()])
        .wait()
        .unwrap();
    let mut read_buf = Bytes::from(read_buf);
    let parsed_cookie: CookieData = Unbuffer::unbuffer_ref(&mut read_buf).unwrap().data();
    check_ver_nonfile_compatible(parsed_cookie.version).unwrap();
}
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
    let conn = connect_tcp(addr)
        .and_then(|tcp_stream| Ok(ConnectionIP::new_client(None, None, tcp_stream, None)))
        .wait()
        .unwrap();
    println!("Hello, world!");
}
