// Copyright 2018-2019, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{constants, Result, VrpnError};
use std::{net::SocketAddr, str::FromStr};
use url::Url;

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
    let server = server.trim_end_matches('/');
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
    type Err = VrpnError;
    fn from_str(url: &str) -> Result<ServerInfo> {
        let urlpart = normalize_scheme(url);

        let parsed = Url::parse(&urlpart)?;

        let scheme = match parsed.scheme() {
            "x-vrpn" => Scheme::UdpAndTcp,
            "tcp" => Scheme::TcpOnly,
            "x-vrsh" => {
                return Err(VrpnError::OtherMessage(format!(
                    "x-vrsh scheme of address {} (url portion {}) not supported",
                    url, urlpart
                )));
            }
            "mpi" => {
                return Err(VrpnError::OtherMessage(format!(
                    "mpi scheme of address {} (url portion {}) not supported",
                    url, urlpart
                )));
            }
            _ => {
                return Err(VrpnError::OtherMessage(format!(
                    "could not parse scheme of address {} (url portion {})",
                    url, urlpart
                )));
            }
        };
        let socket_addr: SocketAddr = parsed
            .socket_addrs(|| Some(constants::DEFAULT_PORT))?
            .into_iter()
            .next()
            .ok_or_else(|| {
                VrpnError::OtherMessage(format!(
                    "could not parse address {} (url portion {})",
                    url, urlpart
                ))
            })?;
        Ok(ServerInfo {
            socket_addr,
            scheme,
        })
    }
}
impl FromStr for DeviceInfo {
    type Err = VrpnError;
    fn from_str(url: &str) -> Result<DeviceInfo> {
        let parts: Vec<&str> = url.split('@').collect();
        let device = match parts.len() {
            1 => None,
            2 => Some(String::from(parts[0])),
            _ => {
                return Err(VrpnError::OtherMessage(format!(
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
    use std::net::ToSocketAddrs;

    use super::*;
    use proptest::prelude::*;

    fn to_addr<T: ToSocketAddrs>(v: T) -> SocketAddr {
        v.to_socket_addrs().unwrap().next().unwrap()
    }

    #[test]
    fn parsing() {
        let _ = "tcp://127.0.0.1:3883".parse::<ServerInfo>().unwrap();
        let _ = "127.0.0.1:3883".parse::<ServerInfo>().unwrap();
        let _ = "x-vrpn:127.0.0.1:3883".parse::<ServerInfo>().unwrap();

        assert!("127.0.0.1:3883".parse::<ServerInfo>().is_ok());
        assert_eq!(
            "127.0.0.1:3883".parse::<ServerInfo>().unwrap(),
            ServerInfo::new(to_addr("127.0.0.1:3883"), Scheme::UdpAndTcp)
        );
        assert_eq!(
            "tcp://127.0.0.1:3883".parse::<ServerInfo>().unwrap(),
            ServerInfo::new(to_addr("127.0.0.1:3883"), Scheme::TcpOnly)
        );
        assert_eq!(
            "tcp://127.0.0.1:3883".parse::<DeviceInfo>().unwrap(),
            DeviceInfo::new(None, to_addr("127.0.0.1:3883"), Scheme::TcpOnly)
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
    proptest! {
        #[test]
        fn noncrash_weird_server(ref s in "\\PC*") {
            let _ = s.parse::<DeviceInfo>();
        }


        #[test]
        fn generated_ip(o0 in 1u8..255, o1 in 1u8..255, o2 in 1u8..255, o3 in 1u8..255, port in 1u16..10000) {
            let proto_and_scheme = [
                ("", Scheme::UdpAndTcp),
                ("x-vrpn:", Scheme::UdpAndTcp),
                ("tcp:", Scheme::TcpOnly)
            ];

            for (proto, scheme) in proto_and_scheme.iter() {

                let ip_string = format!("{}.{}.{}.{}:{}", o0, o1, o2, o3, port);
                // println!("{}", ip_string);
                let ip = to_addr(ip_string.clone());

                let addr_string = format!("{}{}", proto, ip_string);
                // println!("{}", addr_string);
                let parsed = addr_string.parse::<ServerInfo>();
                prop_assert!(parsed.is_ok(), "input string: {}", addr_string);
                let parsed = parsed.unwrap();

                prop_assert_eq!(parsed.socket_addr, ip, "input string: {}", addr_string);
                prop_assert_eq!(parsed.scheme, *scheme, "input string: {}", addr_string);
            }
        }
    }
}
