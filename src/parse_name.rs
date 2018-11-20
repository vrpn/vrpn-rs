// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{constants, Error, Result};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs},
    str::FromStr,
};
use url::Url;

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Error {
        Error::Other(Box::new(e))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Scheme {
    UdpAndTcp,
    TcpOnly,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ServerInfo {
    pub socket_addr: SocketAddr,
    pub scheme: Scheme,
}

impl ServerInfo {
    pub fn new(socket_addr: SocketAddr, scheme: Scheme) -> ServerInfo {
        ServerInfo {
            socket_addr,
            scheme,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct DeviceInfo {
    pub device: Option<String>,
    pub server: ServerInfo,
}

impl DeviceInfo {
    pub fn new(device: Option<String>, socket_addr: SocketAddr, scheme: Scheme) -> DeviceInfo {
        DeviceInfo {
            device,
            server: ServerInfo {
                socket_addr,
                scheme,
            },
        }
    }
}

const SCHEMES: &[&str] = &["x-vrpn:", "x-vrsh:", "tcp:", "mpi:"];

/// Makes sure there's a scheme followed by ://, and ending with a trailing slash.
fn normalize_scheme(server: &str) -> String {
    let server = server.trim_end_matches("/");
    if server.contains("://") {
        // already got a scheme
        return format!("{}/", server);
    }
    for scheme in SCHEMES {
        let s_with_slashes = format!("{}//", *scheme);
        // eprintln!(
        //     "Checking to see if {} starts with {} or {}",
        //     server, *scheme, s_with_slashes
        // );
        if server.starts_with(&s_with_slashes) {
            return format!("{}/", server);
        } else if server.starts_with(*scheme) {
            return format!("{}//{}/", *scheme, server.trim_start_matches(*scheme));
        }
    }
    return format!("x-vrpn://{}/", server);
}

impl FromStr for ServerInfo {
    type Err = Error;
    fn from_str(url: &str) -> Result<ServerInfo> {
        let urlpart = normalize_scheme(url);

        let parsed = Url::parse(&urlpart)?;

        let scheme = match parsed.scheme() {
            "x-vrpn" => Scheme::UdpAndTcp,
            "tcp" => Scheme::TcpOnly,
            "x-vrsh" => {
                return Err(Error::OtherMessage(format!(
                    "x-vrsh scheme of address {} (url portion {}) not supported",
                    url, urlpart
                )));
            }
            "mpi" => {
                return Err(Error::OtherMessage(format!(
                    "mpi scheme of address {} (url portion {}) not supported",
                    url, urlpart
                )));
            }
            _ => {
                return Err(Error::OtherMessage(format!(
                    "could not parse scheme of address {} (url portion {})",
                    url, urlpart
                )));
            }
        };
        let socket_addr = parsed
            .with_default_port(|_| Ok(constants::DEFAULT_PORT))?
            .to_socket_addrs()?
            .next()
            .ok_or(Error::OtherMessage(format!(
                "could not parse address {} (url portion {})",
                url, urlpart
            )))?;
        Ok(ServerInfo {
            socket_addr,
            scheme,
        })
    }
}
impl FromStr for DeviceInfo {
    type Err = Error;
    fn from_str(url: &str) -> Result<DeviceInfo> {
        let parts: Vec<&str> = url.split('@').collect();
        let device = match parts.len() {
            1 => None,
            2 => Some(String::from(parts[0])),
            _ => {
                return Err(Error::OtherMessage(format!(
                    "could not parse address {}",
                    url
                )));
            }
        };
        let server = parts.last().unwrap().parse::<ServerInfo>()?;

        Ok(DeviceInfo { device, server })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parsing() {
        let _ = "tcp://127.0.0.1:3883".parse::<ServerInfo>().unwrap();
        let _ = "127.0.0.1:3883".parse::<ServerInfo>().unwrap();
        let _ = "x-vrpn:127.0.0.1:3883".parse::<ServerInfo>().unwrap();

        assert!("127.0.0.1:3883".parse::<ServerInfo>().is_ok());
        assert_eq!(
            "127.0.0.1:3883".parse::<ServerInfo>().unwrap(),
            ServerInfo::new(
                "127.0.0.1:3883".to_socket_addrs().unwrap().next().unwrap(),
                Scheme::UdpAndTcp
            )
        );
        assert_eq!(
            "tcp://127.0.0.1:3883".parse::<ServerInfo>().unwrap(),
            ServerInfo::new(
                "127.0.0.1:3883".to_socket_addrs().unwrap().next().unwrap(),
                Scheme::TcpOnly
            )
        );
        assert_eq!(
            "tcp://127.0.0.1:3883".parse::<DeviceInfo>().unwrap(),
            DeviceInfo::new(
                None,
                "127.0.0.1:3883".to_socket_addrs().unwrap().next().unwrap(),
                Scheme::TcpOnly
            )
        );
        assert_eq!(
            "127.0.0.1:3883".parse::<ServerInfo>().unwrap(),
            "x-vrpn://127.0.0.1:3883".parse::<ServerInfo>().unwrap(),
        );

        assert_eq!(
            "127.0.0.1:3883".parse::<ServerInfo>().unwrap(),
            "x-vrpn:127.0.0.1:3883".parse::<ServerInfo>().unwrap(),
        );

        assert_eq!(
            "Tracker0@127.0.0.1:3883".parse::<DeviceInfo>().unwrap(),
            DeviceInfo::new(
                Some("Tracker0".into()),
                "127.0.0.1:3883".to_socket_addrs().unwrap().next().unwrap(),
                Scheme::UdpAndTcp
            )
        );
    }
}
