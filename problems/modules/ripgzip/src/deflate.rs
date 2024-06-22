#![forbid(unsafe_code)]

use std::io::BufRead;

use anyhow::{Context, Result};

use crate::bit_reader::BitReader;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default)]
pub struct BlockHeader {
    pub is_final: bool,
    pub compression_type: CompressionType,
}

#[derive(Debug)]
pub enum CompressionType {
    Uncompressed = 0,
    FixedTree = 1,
    DynamicTree = 2,
    Reserved = 3,
}

impl Default for CompressionType {
    fn default() -> Self {
        Self::Uncompressed
    }
}

impl From<u16> for CompressionType {
    fn from(value: u16) -> Self {
        match value {
            0 => Self::Uncompressed,
            1 => Self::FixedTree,
            2 => Self::DynamicTree,
            _ => Self::Reserved,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct DeflateReader<T> {
    bit_reader: BitReader<T>,
    final_reached: bool,
}

impl<T: BufRead> DeflateReader<T> {
    pub fn new(bit_reader: BitReader<T>) -> Self {
        Self {
            bit_reader,
            final_reached: false,
        }
    }

    pub fn next_block(&mut self) -> Option<Result<(BlockHeader, &mut BitReader<T>)>> {
        if self.final_reached {
            return None;
        }

        let mut header = BlockHeader::default();
        match self.bit_reader.read_bits(1) {
            Ok(is_final) => {
                header.is_final = is_final.bits() == 1;
                self.final_reached = header.is_final
            }
            Err(err) => return Some(Err(err).context("reading final bit")),
        }
        match self.bit_reader.read_bits(2) {
            Ok(comp_type) => {
                header.compression_type = comp_type.bits().into();
            }
            Err(err) => return Some(Err(err).context("reading compression type bits")),
        }
        Some(Ok((header, &mut self.bit_reader)))
    }
}

// TODO: your code goes here.
