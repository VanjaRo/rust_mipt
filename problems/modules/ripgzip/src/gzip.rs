#![forbid(unsafe_code)]

use std::io::BufRead;

use anyhow::{anyhow, ensure, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use crc::Crc;

////////////////////////////////////////////////////////////////////////////////

const ID1: u8 = 0x1f;
const ID2: u8 = 0x8b;

const CM_DEFLATE: u8 = 8;

const FTEXT_OFFSET: u8 = 0;
const FHCRC_OFFSET: u8 = 1;
const FEXTRA_OFFSET: u8 = 2;
const FNAME_OFFSET: u8 = 3;
const FCOMMENT_OFFSET: u8 = 4;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MemberHeader {
    pub compression_method: CompressionMethod,
    pub modification_time: u32,
    pub extra: Option<Vec<u8>>,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub extra_flags: u8,
    pub os: u8,
    pub has_crc: bool,
    pub is_text: bool,
}

impl MemberHeader {
    pub fn crc16(&self) -> u16 {
        let crc = Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
        let mut digest = crc.digest();

        digest.update(&[ID1, ID2, self.compression_method.into(), self.flags().0]);
        digest.update(&self.modification_time.to_le_bytes());
        digest.update(&[self.extra_flags, self.os]);

        if let Some(extra) = &self.extra {
            digest.update(&(extra.len() as u16).to_le_bytes());
            digest.update(extra);
        }

        if let Some(name) = &self.name {
            digest.update(name.as_bytes());
            digest.update(&[0]);
        }

        if let Some(comment) = &self.comment {
            digest.update(comment.as_bytes());
            digest.update(&[0]);
        }

        (digest.finalize() & 0xffff) as u16
    }

    pub fn flags(&self) -> MemberFlags {
        let mut flags = MemberFlags(0);
        flags.set_is_text(self.is_text);
        flags.set_has_crc(self.has_crc);
        flags.set_has_extra(self.extra.is_some());
        flags.set_has_name(self.name.is_some());
        flags.set_has_comment(self.comment.is_some());
        flags
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompressionMethod {
    Deflate,
    Unknown(u8),
}

impl From<u8> for CompressionMethod {
    fn from(value: u8) -> Self {
        match value {
            CM_DEFLATE => Self::Deflate,
            x => Self::Unknown(x),
        }
    }
}

impl From<CompressionMethod> for u8 {
    fn from(method: CompressionMethod) -> u8 {
        match method {
            CompressionMethod::Deflate => CM_DEFLATE,
            CompressionMethod::Unknown(x) => x,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MemberFlags(u8);

#[allow(unused)]
impl MemberFlags {
    fn bit(&self, n: u8) -> bool {
        (self.0 >> n) & 1 != 0
    }

    fn set_bit(&mut self, n: u8, value: bool) {
        if value {
            self.0 |= 1 << n;
        } else {
            self.0 &= !(1 << n);
        }
    }

    pub fn is_text(&self) -> bool {
        self.bit(FTEXT_OFFSET)
    }

    pub fn set_is_text(&mut self, value: bool) {
        self.set_bit(FTEXT_OFFSET, value)
    }

    pub fn has_crc(&self) -> bool {
        self.bit(FHCRC_OFFSET)
    }

    pub fn set_has_crc(&mut self, value: bool) {
        self.set_bit(FHCRC_OFFSET, value)
    }

    pub fn has_extra(&self) -> bool {
        self.bit(FEXTRA_OFFSET)
    }

    pub fn set_has_extra(&mut self, value: bool) {
        self.set_bit(FEXTRA_OFFSET, value)
    }

    pub fn has_name(&self) -> bool {
        self.bit(FNAME_OFFSET)
    }

    pub fn set_has_name(&mut self, value: bool) {
        self.set_bit(FNAME_OFFSET, value)
    }

    pub fn has_comment(&self) -> bool {
        self.bit(FCOMMENT_OFFSET)
    }

    pub fn set_has_comment(&mut self, value: bool) {
        self.set_bit(FCOMMENT_OFFSET, value)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MemberFooter {
    pub data_crc32: u32,
    pub data_size: u32,
}

////////////////////////////////////////////////////////////////////////////////

pub struct GzipReader<T> {
    reader: T,
}

impl<T: BufRead> GzipReader<T> {
    pub fn new(reader: T) -> Self {
        Self { reader }
    }

    pub fn next_member(mut self) -> Option<Result<(MemberHeader, MemberReader<T>)>> {
        match self.reader.fill_buf() {
            Ok(buf) => {
                if !buf.is_empty() {
                    Some(self.parse_header())
                } else {
                    None
                }
            }
            Err(err) => Some(Err(anyhow!(err))),
        }
    }

    fn parse_header(mut self) -> Result<(MemberHeader, MemberReader<T>)> {
        // See RFC 1952, section 2.3.
        let id1 = self.reader.read_u8()?;
        let id2 = self.reader.read_u8()?;
        ensure!(id1 == ID1 && id2 == ID2, "wrong id values");

        let compression_method =
            CompressionMethod::from(self.reader.read_u8().context("Compression Method")?);
        ensure!(
            compression_method == CompressionMethod::Deflate,
            "unsupported compression method"
        );

        let flgs = MemberFlags(self.reader.read_u8().context("FLGS")?);

        let is_text = flgs.is_text();
        let mtime = self.reader.read_u32::<LittleEndian>().context("MTIME")?;
        let xfl = self.reader.read_u8().context("eXtra FLags")?;
        let os = self.reader.read_u8().context("Operating System")?;

        let mut extra: Option<Vec<u8>> = None;
        if flgs.has_extra() {
            let xlen = self.reader.read_u16::<LittleEndian>().context("XLEN")?;
            let mut buf = vec![0; xlen as usize];
            self.reader
                .read_exact(buf.as_mut_slice())
                .context("reading extra flags")?;
            extra.replace(buf);
        }

        let mut f_name: Option<String> = None;
        if flgs.has_name() {
            let mut buf: Vec<u8> = vec![];
            self.reader
                .read_until(0, &mut buf)
                .context("reading file name")?;
            f_name.replace(
                String::from_utf8(buf).context("converting file name byte stream to string")?,
            );
        }

        let mut comment: Option<String> = None;
        if flgs.has_comment() {
            let mut buf: Vec<u8> = vec![];
            self.reader
                .read_until(0, &mut buf)
                .context("reading comment")?;
            comment.replace(
                String::from_utf8(buf).context("converting comment byte stream to string")?,
            );
        }

        let has_crc = flgs.has_crc();

        let member_header = MemberHeader {
            compression_method,
            modification_time: mtime,
            extra,
            name: f_name,
            comment,
            extra_flags: xfl,
            os,
            has_crc,
            is_text,
        };

        if flgs.has_crc() {
            let crc16 = self
                .reader
                .read_u16::<LittleEndian>()
                .context("reading crc16")?;
            ensure!(member_header.crc16() == crc16, "header crc16 check failed");
        }
        Ok((member_header, MemberReader { inner: self.reader }))
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct MemberReader<T> {
    inner: T,
}

impl<T: BufRead> MemberReader<T> {
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn read_footer(mut self) -> Result<(MemberFooter, GzipReader<T>)> {
        let data_crc32 = self.inner.read_u32::<LittleEndian>()?;
        let data_size = self.inner.read_u32::<LittleEndian>()?;

        Ok((
            MemberFooter {
                data_crc32,
                data_size,
            },
            GzipReader::new(self.inner),
        ))
    }
}
