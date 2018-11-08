// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;

bitmask!{
    pub mask LogMode: u8 where flags LogFlags {
        NONE = 0,
        INCOMING = (1 << 0),
        OUTGOING = (1 << 1),
        INCOMING_OUTGOING = (1 << 0)|(1 << 1)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LogFileNames {
    pub in_log_file: Option<Bytes>,
    pub out_log_file: Option<Bytes>,
}

fn make_log_name(name: Option<Bytes>) -> Option<Bytes> {
    match name {
        None => None,
        Some(name_str) => {
            if name_str.len() > 0 {
                Some(name_str)
            } else {
                None
            }
        }
    }
}

impl LogFileNames {
    pub fn new() -> LogFileNames {
        LogFileNames {
            out_log_file: None,
            in_log_file: None,
        }
    }
    pub fn from_names(in_log_file: Option<Bytes>, out_log_file: Option<Bytes>) -> LogFileNames {
        LogFileNames {
            out_log_file: make_log_name(out_log_file),
            in_log_file: make_log_name(in_log_file),
        }
    }

    pub fn in_log(&self) -> &Option<Bytes> {
        &self.in_log_file
    }

    pub fn out_log(&self) -> &Option<Bytes> {
        &self.out_log_file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn log_names() {
        assert_eq!(make_log_name(None), None);
        assert_eq!(make_log_name(Some(Bytes::from_static(b""))), None);
        assert_eq!(
            make_log_name(Some(Bytes::from_static(b"asdf"))),
            Some(Bytes::from_static(b"asdf"))
        );
    }
}
