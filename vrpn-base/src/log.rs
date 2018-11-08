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
    in_log_file: Option<Bytes>,
    out_log_file: Option<Bytes>,
}

fn make_log_name<T>(name: Option<T>) -> Option<Bytes>
where
    Bytes: std::convert::From<T>,
{
    match name {
        None => None,
        Some(name_str) => {
            let name_str = Bytes::from(name_str);
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
    pub fn from_names<T>(in_log_file: Option<T>, out_log_file: Option<T>) -> LogFileNames
    where
        Bytes: std::convert::From<T>,
    {
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

    pub fn log_mode(&self) -> LogMode {
        let in_mode = if self.in_log_file.is_some() {
            LogFlags::INCOMING
        } else {
            LogFlags::NONE
        };
        let out_mode = if self.out_log_file.is_some() {
            LogFlags::OUTGOING
        } else {
            LogFlags::NONE
        };
        in_mode | out_mode
    }

    pub fn filenames_iter<'a>(&'a self) -> LogFileNameIter<'a> {
        LogFileNameIter {
            names: self,
            state: Some(FileNameState::In),
        }
    }
}
impl From<Option<LogFileNames>> for LogFileNames {
    fn from(v: Option<LogFileNames>) -> LogFileNames {
        match v {
            None => LogFileNames::new(),
            Some(names) => names,
        }
    }
}
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum FileNameState {
    In,
    Out,
}
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LogFileNameIter<'a> {
    names: &'a LogFileNames,
    state: Option<FileNameState>,
}

impl<'a> Iterator for LogFileNameIter<'a> {
    type Item = &'a Option<Bytes>;
    fn next(&mut self) -> Option<Self::Item> {
        let state = self.state.clone();
        match state {
            None => None,
            Some(FileNameState::In) => {
                // advance
                self.state = Some(FileNameState::Out);
                Some(self.names.in_log())
            }
            Some(FileNameState::Out) => {
                // advance
                self.state = None;
                Some(self.names.out_log())
            }
        }
    }
}

impl Default for LogFileNames {
    fn default() -> LogFileNames {
        LogFileNames::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn log_names() {
        // turbofish required here because None doesn't suggest a type for Some
        assert_eq!(make_log_name::<&[u8]>(None), None);

        assert_eq!(make_log_name(Some(Bytes::from_static(b""))), None);
        assert_eq!(
            make_log_name(Some(Bytes::from_static(b"asdf"))),
            Some(Bytes::from_static(b"asdf"))
        );
    }
    #[test]
    fn log_mode() {
        assert_eq!(LogFileNames::new().log_mode(), LogMode::none());
        assert_eq!(
            LogFileNames::from_names(Some(&b"a"[..]), None).log_mode(),
            LogMode::from(LogFlags::INCOMING)
        );
        assert_eq!(
            LogFileNames::from_names(None, Some(&b"a"[..])).log_mode(),
            LogMode::from(LogFlags::OUTGOING)
        );
        assert_eq!(
            LogFileNames::from_names(Some(&b"a"[..]), Some(&b"a"[..])).log_mode(),
            LogMode::from(LogFlags::INCOMING_OUTGOING)
        );
    }
}
