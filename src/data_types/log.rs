// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::buffer_unbuffer::{
    buffer, size_requirement::*, unbuffer, BufferSize, BufferUnbufferError, ConstantBufferSize,
};
use bytes::{Buf, BufMut, Bytes};

use super::{constants, MessageTypeIdentifier, TypedMessageBody};

bitflags! {
    pub struct LogMode: u8  {
        const NONE = 0;
        const INCOMING = (1 << 0);
        const OUTGOING = (1 << 1);
        const INCOMING_OUTGOING = (1 << 0)|(1 << 1);
    }
}

/// Stores an optional byte string for log file name, one for in, one for out.
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
            if !name_str.is_empty() {
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
            LogMode::INCOMING
        } else {
            LogMode::NONE
        };
        let out_mode = if self.out_log_file.is_some() {
            LogMode::OUTGOING
        } else {
            LogMode::NONE
        };
        in_mode | out_mode
    }

    pub fn filenames_iter(&'_ self) -> LogFileNameIter<'_> {
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

/// Allows iteration through the two optional fields of the LogFileNames struct.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LogFileNameIter<'a> {
    names: &'a LogFileNames,
    state: Option<FileNameState>,
}

impl<'a> Iterator for LogFileNameIter<'a> {
    type Item = &'a Option<Bytes>;
    fn next(&mut self) -> Option<Self::Item> {
        let state = self.state;
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

fn filename_len(filename: &Option<Bytes>) -> usize {
    match filename {
        Some(name) => name.len(),
        None => 0,
    }
}

impl BufferSize for LogFileNames {
    fn buffer_size(&self) -> usize {
        2 + // null terminators
        2 * u32::constant_buffer_size()  +
        self.filenames_iter().fold(0_usize, |acc, name| acc + filename_len(name))
    }
}

impl buffer::BufferTo for LogFileNames {
    fn buffer_to<T: BufMut>(&self, buf: &mut T) -> buffer::BufferResult {
        buffer::check_buffer_remaining(buf, self.buffer_size())?;
        for filename in self.filenames_iter() {
            (filename_len(filename) as i32).buffer_to(buf)?;
        }
        for filename in self.filenames_iter() {
            if let Some(name) = filename {
                buf.put_slice(name);
            }
            buf.put_u8(0);
        }
        Ok(())
    }
}

impl TypedMessageBody for LogFileNames {
    const MESSAGE_IDENTIFIER: MessageTypeIdentifier =
        MessageTypeIdentifier::SystemMessageId(constants::LOG_DESCRIPTION);
}

fn unbuffer_logname<T: Buf>(len: usize, buf: &mut T) -> unbuffer::UnbufferResult<Option<Bytes>> {
    let name = if len > 0 {
        Some(buf.copy_to_bytes(len))
    } else {
        None
    };

    unbuffer::consume_expected(buf, b"\0")?;
    Ok(name)
}

impl unbuffer::UnbufferFrom for LogFileNames {
    fn unbuffer_from<T: Buf>(buf: &mut T) -> unbuffer::UnbufferResult<Self> {
        let min_size = 2 * u32::constant_buffer_size() + 2;
        if buf.remaining() < min_size {
            return Err(BufferUnbufferError::NeedMoreData(SizeRequirement::AtLeast(
                min_size - buf.remaining(),
            )));
        }
        let in_len = u32::unbuffer_from(buf)?;
        let out_len = u32::unbuffer_from(buf)?;

        let in_name = unbuffer_logname(in_len as usize, buf)?;
        let out_name = unbuffer_logname(out_len as usize, buf)?;

        Ok(LogFileNames::from_names(in_name, out_name))
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
        assert_eq!(LogFileNames::new().log_mode(), LogMode::NONE);
        assert_eq!(
            LogFileNames::from_names(Some(&b"a"[..]), None).log_mode(),
            LogMode::INCOMING
        );
        assert_eq!(
            LogFileNames::from_names(None, Some(&b"a"[..])).log_mode(),
            LogMode::OUTGOING
        );
        assert_eq!(
            LogFileNames::from_names(Some(&b"a"[..]), Some(&b"a"[..])).log_mode(),
            LogMode::INCOMING_OUTGOING
        );
    }
}
