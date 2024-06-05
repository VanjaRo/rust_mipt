#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::io::{self, Write};

use anyhow::{anyhow, ensure, Context, Result};
use crc::{Crc, Digest, CRC_32_ISO_HDLC};

////////////////////////////////////////////////////////////////////////////////

const HISTORY_SIZE: usize = 32768;
const ALGORITHM: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

pub struct TrackingWriter<T> {
    inner: T,
    history_buff: VecDeque<u8>,
    byte_counter: usize,
    digest: Digest<'static, u32>,
}

impl<T: Write> Write for TrackingWriter<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written_bytes = self.inner.write(buf)?;
        self.byte_counter += written_bytes;
        self.digest.update(&buf[..written_bytes]);
        let mut drain_val = 0;
        if written_bytes >= HISTORY_SIZE {
            drain_val = self.history_buff.len();
        } else if self.history_buff.len().wrapping_add(written_bytes) > HISTORY_SIZE {
            drain_val = self.history_buff.len().wrapping_add(written_bytes) - HISTORY_SIZE;
        }
        self.history_buff.drain(0..drain_val);
        // preventing possible write bigger than HISTORY_SIZE
        let iter_st = 0.max(written_bytes.saturating_sub(HISTORY_SIZE));
        self.history_buff.extend(&buf[iter_st..written_bytes]);

        Ok(written_bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<T: Write> TrackingWriter<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            history_buff: VecDeque::with_capacity(HISTORY_SIZE),
            byte_counter: 0,
            digest: ALGORITHM.digest(),
        }
    }

    /// Write a sequence of `len` bytes written `dist` bytes ago.
    pub fn write_previous(&mut self, dist: usize, len: usize) -> Result<()> {
        let hb_len = self.history_buff.len();
        // println!("{}", hb_len);
        ensure!(
            hb_len >= dist,
            "dist should be less than or equal to the history of previous writes"
        );
        ensure!(len <= dist, "len should be less than dist");

        let iter_st = hb_len - dist;
        self.write_all(
            self.history_buff
                .range(iter_st..iter_st + len)
                .copied()
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .context("write_all failed")
    }

    pub fn byte_count(&self) -> usize {
        self.byte_counter
    }

    pub fn crc32(mut self) -> u32 {
        self.digest.finalize()
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::WriteBytesExt;

    #[test]
    fn write() -> Result<()> {
        let mut buf: &mut [u8] = &mut [0u8; 10];
        let mut writer: TrackingWriter<&mut [u8]> = TrackingWriter::new(buf);

        assert_eq!(writer.write(&[1, 2, 3, 4])?, 4);
        assert_eq!(writer.byte_count(), 4);

        assert_eq!(writer.write(&[4, 8, 15, 16, 23])?, 5);
        assert_eq!(writer.byte_count(), 9);

        assert_eq!(writer.write(&[0, 0, 123])?, 1);
        assert_eq!(writer.byte_count(), 10);

        assert_eq!(writer.write(&[42, 124, 234, 27])?, 0);
        assert_eq!(writer.byte_count(), 10);
        assert_eq!(writer.crc32(), 2992191065);

        Ok(())
    }

    #[test]
    fn write_previous() -> Result<()> {
        let mut buf: &mut [u8] = &mut [0u8; 512];
        let mut writer = TrackingWriter::new(&mut buf);

        for i in 0..=255 {
            writer.write_u8(i)?;
        }

        writer.write_previous(192, 128)?;
        assert_eq!(writer.byte_count(), 384);

        assert!(writer.write_previous(10000, 20).is_err());
        assert_eq!(writer.byte_count(), 384);

        assert!(writer.write_previous(256, 256).is_err());
        assert_eq!(writer.byte_count(), 512);

        assert!(writer.write_previous(1, 1).is_err());
        assert_eq!(writer.byte_count(), 512);
        assert_eq!(writer.crc32(), 2733545866);

        Ok(())
    }
}
