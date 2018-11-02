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

// impl From<VersionError> for ConnectError {
//     fn from(v: VersionError) -> ConnectError {
//         ConnectError::VersionError(v)
//     }
// }
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
        item.buffer_ref(dst)
            .map_err(|e| ConnectError::BufferError(e))
    }
}
impl<T: Unbuffer> Decoder for CodecWrapper<T> {
    type Item = T;
    type Error = ConnectError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let src_len = src.len();
        let mut frozen = src.clone().freeze();
        match T::unbuffer_ref(&mut frozen) {
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
fn _convert_err<T: std::error::Error>(e: T) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}
fn convert_err<T>(e: T) -> ConnectError
where
    T: std::error::Error,
    ConnectError: From<T>,
{
    From::from(e)
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
    let buflen = CookieData::constant_buffer_size();
    let mut send_buf = BytesMut::with_capacity(buflen);
    let cookie = CookieData::from(constants::MAGIC_DATA);
    cookie.buffer_ref(&mut send_buf).unwrap();
    // if let Err(e) = cookie.buffer_ref(&mut send_buf) {
    //     return Err(ConnectError::BufferError(e));
    // }
    let sock = make_tcp_socket(addr).expect("failure making the socket");

    let stream_future = TcpStream::connect_std(sock, &addr, &tokio::reactor::Handle::default());
    let handshake_future = stream_future
        .or_else(|e| {
            eprintln!("connect error {}", e);
            future::err(ConnectError::IoError(e))
        })
        .and_then(|stream| io::write_all(stream, send_buf).map_err(convert_err))
        .and_then(|(stream, _)| io::read_exact(stream, vec![0u8; buflen]).map_err(convert_err))
        .and_then(|(stream, read_buf)| {
            let mut read_buf = Bytes::from(read_buf);
            CookieData::unbuffer_ref(&mut read_buf)
                .map_err(|e| ConnectError::UnbufferError(e))
                .and_then(|Output(parsed)| {
                    check_ver_nonfile_compatible(parsed.version).map_err(convert_err)
                })
                .and_then(|()| future::ok(stream))
        });
    // .and_then(|(stream, Output(parsed_cookie))| check_ver_nonfile_compatible(parsed_cookie.version).and_then(|()| stream));

    //         if let Err(e) = check_ver_nonfile_compatible(parsed_cookie.version) {
    //             return future::err(ConnectError::VersionError(e));
    //         }
    //         // Ok(stream)
    //     })

    // cookie_codec()
    //     .framed(stream)
    //     .send(CookieData::from(constants::MAGIC_DATA))
    //     .and_then(|s| {
    //         s.into_future().then(|result| {
    //             println!("read; {:?}", result);
    //             // let (Some(data), _) = result?;
    //             // check_ver_nonfile_compatible(data.version)?;
    //             Ok(stream)
    //         })
    //     })

    let client = handshake_future.map_err(|e| eprintln!("got error {}", e));
    tokio::run(client);
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
