extern crate mio;
extern crate socket2;
extern crate tokio;
extern crate vrpn;

#[macro_use]
extern crate futures;

use tokio::io;
use tokio::io::AsyncRead;
use tokio::net::TcpStream;
use tokio::prelude::*;
//use tokio::prelude::{Future, Sink, Stream};
use vrpn::codec::{check_ver_nonfile_compatible, MagicCookie};
use vrpn::translationtable::TranslationTable;
use vrpn::types::{CookieData, LocalId, RemoteId, SenderId};
use vrpn::*;

pub use tokio::codec::{Decoder, Encoder};

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
            sock.bind(&socket2::SockAddr::from(addr));
        } else {
            unimplemented!();
        }
    }
    sock.set_reuse_address(true)?;
    Ok(sock.into_tcp_stream())
}

pub fn connect_tcp(
    addr: std::net::SocketAddr,
) -> impl Future<Item = tokio::net::TcpStream, Error = io::Error> {
    let sock = match make_tcp_socket(addr) {
        Err(e) => {
            return future::err(e);
        }
        Ok(socket) => socket,
    };

    let stream_future = TcpStream::connect_std(sock, &addr, &tokio::reactor::Handle::default());
    let handshake_future = stream_future
        .and_then(|stream| {
            let inner =
                MagicCookie
                    .framed(stream)
                    .send(CookieData::from(constants::MAGIC_DATA))
                    .and_then(|s| {
                        s.into_future()
                            .then(
                                |result| -> std::result::Result<
                                    tokio::net::TcpStream,
                                    codec::CodecError,
                                > {
                                    println!("read; {:?}", result);
                                    match result {
                                        Ok((Some(data), _)) => {
                                            match check_ver_nonfile_compatible(data.version) {
                                                Err(e) => Err(e),
                                                Ok(()) => Ok(stream),
                                            }
                                        }
                                        Err((e, _)) => Err(e),
                                    }
                                },
                            ).map_err(|e| io::Error::new(tokio::io::ErrorKind::InvalidData, e))
                    });
            return inner;
        });
    return handshake_future;
}

#[test]
fn main() {
    let addr = "127.0.0.1:3883".parse().unwrap();
    let client = connect_tcp(addr).map_err(|err| println!("Connection error = {:?}", err));
    tokio::run(client);
    let mut table: TranslationTable<SenderId> = TranslationTable::new();
    table
        .add_remote_entry("asdf", RemoteId(SenderId(0)), LocalId(SenderId(0)))
        .expect("Failed adding remote entry");
    let mut conn = ConnectionIP::new_client(None, None);
    println!("Hello, world!");
}
