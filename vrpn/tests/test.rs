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
use tokio::{
    codec::{self, Decoder, Encoder},
    io::{self, AsyncRead},
    net::TcpStream,
    prelude::*,
};
use vrpn::{
    base::{
        cookie::{self, check_ver_nonfile_compatible, CookieData},
        types::{LocalId, RemoteId, SenderId},
    },
    buffer::{buffer, unbuffer, Buffer, BufferSize, ConstantBufferSize, Output, Unbuffer},
    connection::translationtable::TranslationTable,
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
#[derive(Debug, Copy, Clone)]
struct CodecWrapper<T>(std::marker::PhantomData<T>);

impl<T> CodecWrapper<T> {
    pub fn new() -> CodecWrapper<T> {
        CodecWrapper(Default::default())
    }
}

impl<T> Default for CodecWrapper<T> {
    fn default() -> CodecWrapper<T> {
        CodecWrapper::new()
    }
}

impl<T: Buffer> Encoder for CodecWrapper<T> {
    type Item = T;
    type Error = ConnectError;
    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(item.buffer_size());
        item.buffer(dst).map_err(|e| ConnectError::BufferError(e))
    }
}
impl<T: Unbuffer> Decoder for CodecWrapper<T> {
    type Item = T;
    type Error = ConnectError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let src_len = src.len();
        let mut frozen = src.clone().freeze();
        match T::unbuffer(&mut frozen) {
            Ok(Output(v)) => {
                src.advance(src_len - frozen.len());
                Ok(Some(v))
            }
            Err(unbuffer::Error::NeedMoreData(_)) => Ok(None),
            Err(e) => Err(ConnectError::UnbufferError(e)),
        }
    }
}

fn cookie_codec() -> CodecWrapper<CookieData> {
    Default::default()
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
            sock.bind(&socket2::SockAddr::from(addr));
        } else {
            unimplemented!();
        }
    }
    sock.set_reuse_address(true)?;
    Ok(sock.into_tcp_stream())
}

//impl<T>
fn convert_err<T: std::error::Error>(e: T) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

pub fn connect_tcp(addr: std::net::SocketAddr)
//-> impl Future<Item = tokio::net::TcpStream, Error = io::Error>
{
    /*
    let sock = match make_tcp_socket(addr) {
        Err(e) => {
            return future::err(e);
        }
        Ok(socket) => socket,
    };
    */
    let sock = make_tcp_socket(addr).expect("failure making the socket");

    let stream_future = TcpStream::connect_std(sock, &addr, &tokio::reactor::Handle::default());
    // let handshake_future = stream_future.and_then(|stream| {
    //     let cookie = CookieData::from(constants::MAGIC_DATA);

    //     let send_buf = BytesMut::with_capacity(cookie.buffer_size());
    //     let read_buf = BytesMut::with_capacity(CookieData::constant_buffer_size());
    //     cookie
    //         .buffer(send_buf)
    //         .and_then(|()| stream.send(send_buf))
    //         .and_then(|_| stream.read_buf(&read_buf)).and_then(|_| {
    //          let read_cookie =    Unbuffer::unbuffer(read_buf.freeze())?;
    //         })

    //     cookie_codec()
    //         .framed(stream)
    //         .send(CookieData::from(constants::MAGIC_DATA))
    // .and_then(|s| {
    //     s.into_future().then(|result| {
    //         println!("read; {:?}", result);
    //         // let (Some(data), _) = result?;
    //         // check_ver_nonfile_compatible(data.version)?;
    //         Ok(stream)
    //     })
    // })
    // });
    // let client = handshake_future.map_err(|e| eprintln!("got error {}", e));
    // tokio::run(client);
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
    cookie.buffer(&mut send_buf).unwrap();
    let (stream, _) = io::write_all(stream, send_buf.freeze()).wait().unwrap();
    
    let (_stream, read_buf) = io::read_exact(stream, vec![0u8; CookieData::constant_buffer_size()]).wait().unwrap();
    let mut read_buf = Bytes::from(read_buf);
    let parsed_cookie : CookieData = Unbuffer::unbuffer(&mut read_buf).unwrap().data();
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
