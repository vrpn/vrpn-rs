extern crate bytes;
extern crate mio;
extern crate socket2;
extern crate tokio;
extern crate vrpn;

#[macro_use]
extern crate futures;
#[macro_use]
extern crate quick_error;

use bytes::{Bytes, BytesMut};
use tokio::{io, net::TcpStream, prelude::*};
use vrpn::{
    base::{
        cookie::{self, check_ver_nonfile_compatible, CookieData},
        types::{LocalId, RemoteId, SenderId},
    },
    buffer::{buffer, unbuffer, Buffer, BufferSize, ConstantBufferSize, Output, Unbuffer},
    connection::translationtable::TranslationTable,
    prelude::*,
    *,
};

quick_error! {
    #[derive(Debug)]
    pub enum ConnectError {
        VersionError(err: cookie::VersionError) {
            from()
            display("version error: {}", err)
            cause(err)
        }
        UnbufferError(err: unbuffer::Error) {
            from()
            display("unbuffer error: {}", err)
            cause(err)
        }
        BufferError(err: buffer::Error) {
            from()
            display("buffer error: {}", err)
            cause(err)
        }
        IoError(err: io::Error) {
            from()
            display("IO error: {}", err)
            cause(err)
        }
    }
}

fn make_tcp_socket(addr: std::net::SocketAddr) -> io::Result<std::net::TcpStream> {
    use socket2::*;
    let domain = if addr.is_ipv4() {
        Domain::ipv4()
    } else {
        Domain::ipv6()
    };
    let sock = socket2::Socket::new(domain, Type::stream(), Some(Protocol::tcp()))?;
    sock.set_nonblocking(true)?;
    sock.set_nodelay(true)?;

    if cfg!(windows) {
        if addr.is_ipv4() {
            let any = std::net::Ipv4Addr::new(0, 0, 0, 0);
            let addr = std::net::SocketAddrV4::new(any, 0);
            sock.bind(&socket2::SockAddr::from(addr))?;
        } else {
            unimplemented!();
        }
    }
    sock.set_reuse_address(true)?;
    Ok(sock.into_tcp_stream())
}

fn convert_err<T>(e: T) -> ConnectError
where
    T: std::error::Error,
    ConnectError: From<T>,
{
    From::from(e)
}

pub fn connect_tcp(
    addr: std::net::SocketAddr,
) -> impl Future<Item = tokio::net::TcpStream, Error = ConnectError> {
    let sock = make_tcp_socket(addr).expect("failure making the socket");

    let stream_future = TcpStream::connect_std(sock, &addr, &tokio::reactor::Handle::default());
    stream_future
        .or_else(|e| {
            eprintln!("connect error {}", e);
            future::err(ConnectError::IoError(e))
        })
        .and_then(|stream| {
            let cookie = CookieData::from(constants::MAGIC_DATA);
            BytesMut::new()
                .allocate_and_buffer(cookie)
                .map_err(convert_err)
                .into_future()
                .and_then(|buf| io::write_all(stream, buf.freeze()).map_err(convert_err))
        })
        .and_then(|(stream, _)| {
            io::read_exact(stream, vec![0u8; CookieData::constant_buffer_size()])
                .map_err(convert_err)
        })
        .and_then(|(stream, read_buf)| {
            println!("{:?}", stream);
            let mut read_buf = Bytes::from(read_buf);
            CookieData::unbuffer_ref(&mut read_buf)
                .map_err(|e| ConnectError::UnbufferError(e))
                .and_then(|Output(parsed)| {
                    check_ver_nonfile_compatible(parsed.version).map_err(convert_err)
                })
                .and_then(|()| Ok(stream))
        })
}
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
    connect_tcp(addr);
    //let client = connect_tcp(addr).map_err(|err| println!("Connection error = {:?}", err));
    //tokio::run(client);
    let mut table: TranslationTable<SenderId> = TranslationTable::new();
    table
        .add_remote_entry(
            Bytes::from_static(b"asdf"),
            RemoteId(SenderId(0)),
            LocalId(SenderId(0)),
        )
        .expect("Failed adding remote entry");
    let mut conn = ConnectionIP::new_client(None, None);
    println!("Hello, world!");
}
