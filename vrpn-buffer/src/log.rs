// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{BufMut, Bytes};
use crate::{check_expected, Buffer, BufferSize, ConstantBufferSize, Unbuffer};
use vrpn_base::{BytesRequired, EmptyResult, Error, LogFileNames, Result};

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
        self.filenames_iter().fold(0 as usize, |acc, name| acc + filename_len(name))
    }
}
impl Buffer for LogFileNames {
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> EmptyResult {
        if buf.remaining_mut() < self.buffer_size() {
            return Err(Error::OutOfBuffer);
        }
        for filename in self.filenames_iter() {
            (filename_len(filename) as i32).buffer_ref(buf)?;
        }
        for filename in self.filenames_iter() {
            if let Some(name) = filename {
                buf.put(name);
            }
            buf.put_u8(0);
        }
        Ok(())
    }
}

fn unbuffer_logname(len: usize, buf: &mut Bytes) -> Result<Option<Bytes>> {
    let name = if len > 0 {
        Some(buf.split_to(len))
    } else {
        None
    };

    check_expected(buf, b"\0")?;
    Ok(name)
}

impl Unbuffer for LogFileNames {
    fn unbuffer_ref(buf: &mut Bytes) -> Result<LogFileNames> {
        let min_size = 2 * u32::constant_buffer_size() + 2;
        if buf.len() < min_size {
            Err(Error::NeedMoreData(BytesRequired::AtLeast(
                min_size - buf.len(),
            )))?;
        }
        let in_len = u32::unbuffer_ref(buf)?;
        let out_len = u32::unbuffer_ref(buf)?;

        let in_name = unbuffer_logname(in_len as usize, buf)?;
        let out_name = unbuffer_logname(out_len as usize, buf)?;

        Ok(LogFileNames::from_names(in_name, out_name))
    }
}
