#![forbid(unsafe_code)]

use std::io::{BufRead, Write};

use anyhow::{bail, ensure, Context, Result};
use bit_reader::BitReader;
use byteorder::{LittleEndian, ReadBytesExt};
use deflate::{CompressionType, DeflateReader};
use huffman_coding::{decode_litlen_distance_trees, LitLenToken};
use tracking_writer::TrackingWriter;

use crate::gzip::GzipReader;

mod bit_reader;
mod deflate;
mod gzip;
mod huffman_coding;
mod tracking_writer;

pub fn decompress<R: BufRead, W: Write>(input: R, output: W) -> Result<()> {
    let mut gzip_reader = GzipReader::new(input);
    let mut tracking_writer = TrackingWriter::new(output);
    while let Some(member) = gzip_reader.next_member() {
        let (_, mut member_reader) = member?;
        let mut reader = DeflateReader::new(BitReader::new(member_reader.inner_mut()));
        while let Some(block) = reader.next_block() {
            let (header, r) = block?;
            match header.compression_type {
                CompressionType::Uncompressed => {
                    let stream = r.borrow_reader_from_boundary();

                    let len = stream
                        .read_u16::<LittleEndian>()
                        .context("reading len of block")?;
                    let nlen = stream
                        .read_u16::<LittleEndian>()
                        .context("reading nlen of block")?;
                    ensure!(len == !nlen, "nlen check failed");

                    for _ in 0..len {
                        tracking_writer.write_all(&[stream.read_u8()?])?;
                    }
                }
                CompressionType::DynamicTree => {
                    let (litlen_coding, distance_coding) = decode_litlen_distance_trees(r)?;
                    loop {
                        let token = litlen_coding.read_symbol(r)?;
                        match token {
                            LitLenToken::Literal(lit) => {
                                tracking_writer.write_all(&[lit])?;
                            }
                            LitLenToken::EndOfBlock => break,
                            LitLenToken::Length { base, extra_bits } => {
                                let len = (base + r.read_bits(extra_bits)?.bits()) as usize;
                                let distance_token = distance_coding.read_symbol(r)?;
                                let distance = (distance_token.base
                                    + r.read_bits(distance_token.extra_bits)?.bits())
                                    as usize;
                                tracking_writer.write_previous(distance, len)?;
                            }
                        }
                    }
                }
                _ => bail!("unsupported block type"),
            }
            if header.is_final {
                break;
            }
        }
        let (footer, new_gzip_reader) = member_reader.read_footer().context("footer error")?;
        ensure!(
            footer.data_size as usize == tracking_writer.byte_count(),
            "length check failed"
        );
        ensure!(
            footer.data_crc32 == tracking_writer.crc32(),
            "crc32 check failed"
        );
        gzip_reader = new_gzip_reader;
        tracking_writer.flush()?;
    }
    Ok(())
}
