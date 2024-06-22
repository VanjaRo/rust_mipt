#![forbid(unsafe_code)]

use std::{collections::HashMap, convert::TryFrom, io::BufRead, usize};

use anyhow::{bail, ensure, Ok, Result};

use crate::bit_reader::{BitReader, BitSequence};

////////////////////////////////////////////////////////////////////////////////

pub fn decode_litlen_distance_trees<T: BufRead>(
    bit_reader: &mut BitReader<T>,
) -> Result<(HuffmanCoding<LitLenToken>, HuffmanCoding<DistanceToken>)> {
    // See RFC 1951, section 3.2.7.
    let hlit = bit_reader.read_bits(5)?.bits();
    let hdist = bit_reader.read_bits(5)?.bits();
    let hclen = bit_reader.read_bits(4)?.bits();

    let cl_hcoding = decode_codelen_token(bit_reader, hclen)?;
    let litlen_hcoding = decode_litlen_token(bit_reader, hlit, &cl_hcoding)?;
    let dist_hcoding = decode_dist_token(bit_reader, hdist, &cl_hcoding)?;

    Ok((litlen_hcoding, dist_hcoding))
}

const RFC_CODE_LENGHTHS_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

fn decode_codelen_token<T: BufRead>(
    bit_reader: &mut BitReader<T>,
    hclen: u16,
) -> Result<HuffmanCoding<TreeCodeToken>> {
    let mut code_lengths: [u8; 19] = [0; 19];

    for &pos in &RFC_CODE_LENGHTHS_ORDER[..(hclen + 4).into()] {
        code_lengths[pos] = bit_reader.read_bits(3)?.bits() as u8;
    }
    HuffmanCoding::from_lengths(&code_lengths)
}

fn decode_litlen_token<T: BufRead>(
    bit_reader: &mut BitReader<T>,
    hlit: u16,
    cl_hcoding: &HuffmanCoding<TreeCodeToken>,
) -> Result<HuffmanCoding<LitLenToken>> {
    let mut litlen_tokens: [u8; 286] = [0; 286];

    decode_cl_alphabt(
        &mut litlen_tokens,
        bit_reader,
        (hlit + 257) as usize,
        cl_hcoding,
    )?;

    HuffmanCoding::from_lengths(&litlen_tokens)
}

fn decode_dist_token<T: BufRead>(
    bit_reader: &mut BitReader<T>,
    hdist: u16,
    cl_hcoding: &HuffmanCoding<TreeCodeToken>,
) -> Result<HuffmanCoding<DistanceToken>> {
    let mut dist_tokens: [u8; 32] = [0; 32];

    decode_cl_alphabt(
        &mut dist_tokens,
        bit_reader,
        (hdist + 1) as usize,
        cl_hcoding,
    )?;

    HuffmanCoding::from_lengths(&dist_tokens)
}
fn decode_cl_alphabt<T: BufRead>(
    cl_vals: &mut [u8],
    bit_reader: &mut BitReader<T>,
    cl_vals_count: usize,
    cl_hcoding: &HuffmanCoding<TreeCodeToken>,
) -> Result<()> {
    let mut pos = 0;
    while pos < cl_vals_count {
        let token = cl_hcoding.read_symbol(bit_reader)?;
        match token {
            TreeCodeToken::Length(t_len) => {
                cl_vals[pos] = t_len;
                pos += 1;
            }
            TreeCodeToken::CopyPrev => {
                let repeat_times = bit_reader.read_bits(2)?.bits().wrapping_add(3);
                for i in 0..repeat_times as usize {
                    cl_vals[pos] = cl_vals[pos - 1 - i];
                    pos += 1;
                }
            }
            TreeCodeToken::RepeatZero { base, extra_bits } => {
                let repeat_times = bit_reader.read_bits(extra_bits)?.bits().wrapping_add(base);

                for i in 0..repeat_times as usize {
                    cl_vals[pos + i] = 0;
                }
                pos += repeat_times as usize;
            }
        }
    }
    Ok(())
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
pub enum TreeCodeToken {
    Length(u8),
    CopyPrev,
    RepeatZero { base: u16, extra_bits: u8 },
}

impl TryFrom<HuffmanCodeWord> for TreeCodeToken {
    type Error = anyhow::Error;

    fn try_from(value: HuffmanCodeWord) -> Result<Self> {
        // See RFC 1951, section 3.2.7.
        match value.0 {
            0..=15 => Ok(Self::Length(value.0 as u8)),
            16 => Ok(Self::CopyPrev),
            17 => Ok(Self::RepeatZero {
                base: 3,
                extra_bits: 3,
            }),
            18 => Ok(Self::RepeatZero {
                base: 11,
                extra_bits: 7,
            }),
            _ => bail!("Unexped value for the TreeCodeToken"),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
pub enum LitLenToken {
    Literal(u8),
    EndOfBlock,
    Length { base: u16, extra_bits: u8 },
}

impl TryFrom<HuffmanCodeWord> for LitLenToken {
    type Error = anyhow::Error;

    fn try_from(value: HuffmanCodeWord) -> Result<Self> {
        // See RFC 1951, section 3.2.5.
        match value.0 {
            0..=255 => Ok(Self::Literal(value.0 as u8)),

            256 => Ok(Self::EndOfBlock),

            257..=264 => Ok(Self::Length {
                base: value.0 - 254,
                extra_bits: 0,
            }),
            265..=268 => Ok(Self::Length {
                base: 11 + 2 * (value.0 - 265),
                extra_bits: 1,
            }),
            269..=272 => Ok(Self::Length {
                base: 19 + 4 * (value.0 - 269),
                extra_bits: 2,
            }),
            273..=276 => Ok(Self::Length {
                base: 35 + 8 * (value.0 - 273),
                extra_bits: 3,
            }),
            277..=280 => Ok(Self::Length {
                base: 67 + 16 * (value.0 - 277),
                extra_bits: 4,
            }),
            281..=284 => Ok(Self::Length {
                base: 131 + 32 * (value.0 - 281),
                extra_bits: 5,
            }),
            285 => Ok(Self::Length {
                base: 258,
                extra_bits: 0,
            }),
            _ => bail!("Unexped value for the LitLenToken"),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
pub struct DistanceToken {
    pub base: u16,
    pub extra_bits: u8,
}

// Array of tuples representing (base, extra_bits) values for each HuffmanCodeWord
#[rustfmt::skip]
const DISTANCE_CODES: [(u16, u8); 30] = [
            (1, 0), (2, 0), (3, 0), (4, 0),  // 0-3
            (5, 1), (7, 1),                  // 4-5
            (9, 2), (13, 2),                 // 6-7
            (17, 3), (25, 3),                // 8-9
            (33, 4), (49, 4),                // 10-11
            (65, 5), (97, 5),                // 12-13
            (129, 6), (193, 6),              // 14-15
            (257, 7), (385, 7),              // 16-17
            (513, 8), (769, 8),              // 18-19
            (1025, 9), (1537, 9),            // 20-21
            (2049, 10), (3073, 10),          // 22-23
            (4097, 11), (6145, 11),          // 24-25
            (8193, 12), (12289, 12),         // 26-27
            (16385, 13), (24577, 13),        // 28-29
        ];

impl TryFrom<HuffmanCodeWord> for DistanceToken {
    type Error = anyhow::Error;

    fn try_from(value: HuffmanCodeWord) -> Result<Self> {
        // See RFC 1951, section 3.2.5.
        if let Some(&(base, extra_bits)) = &DISTANCE_CODES.get(value.0 as usize) {
            Ok(Self { base, extra_bits })
        } else {
            bail!("Unexpected value for the DistanceToken")
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

const MAX_BITS: usize = 15;

pub struct HuffmanCodeWord(pub u16);

pub struct HuffmanCoding<T> {
    map: HashMap<BitSequence, T>,
}

impl<T> HuffmanCoding<T>
where
    T: Copy + TryFrom<HuffmanCodeWord, Error = anyhow::Error>,
{
    pub fn new(map: HashMap<BitSequence, T>) -> Self {
        Self { map }
    }

    #[allow(unused)]
    pub fn decode_symbol(&self, seq: BitSequence) -> Option<T> {
        self.map.get(&seq).copied()
    }

    pub fn read_symbol<U: BufRead>(&self, bit_reader: &mut BitReader<U>) -> Result<T> {
        let mut bit_sequence = BitSequence::new(0, 0);

        for _ in 0..MAX_BITS {
            let bit = bit_reader.read_bits(1)?;
            bit_sequence = bit_sequence.concat(bit);
            if let Some(value) = self.decode_symbol(bit_sequence) {
                return Ok(value);
            }
        }
        bail!("no suitable symbol to decode")
    }

    pub fn from_lengths(code_lengths: &[u8]) -> Result<Self> {
        // See RFC 1951, section 3.2.2.
        ensure!(code_lengths.len() <= u16::MAX as usize);

        let mut bl_count: [u16; MAX_BITS + 1] = [0; MAX_BITS + 1];
        let mut next_code: [u16; MAX_BITS + 1] = [0; MAX_BITS + 1];

        for &bl in code_lengths {
            ensure!(bl as usize <= MAX_BITS);
            bl_count[bl as usize] += 1;
        }

        let mut code = 0;
        bl_count[0] = 0;
        for bits in 1..=MAX_BITS {
            code = (code + bl_count[bits - 1]) << 1;
            next_code[bits] = code;
        }
        let mut huffman_map = HashMap::<BitSequence, T>::new();
        for (idx, &len) in code_lengths.iter().enumerate() {
            if len != 0 {
                let bits_val = next_code[len as usize];
                let bit_seq = BitSequence::new(bits_val, len);
                let val_code_word = T::try_from(HuffmanCodeWord(idx as u16))?;

                huffman_map.insert(bit_seq, val_code_word);
                next_code[len as usize] += 1;
            }
        }

        Ok(HuffmanCoding::new(huffman_map))
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Value(u16);

    impl TryFrom<HuffmanCodeWord> for Value {
        type Error = anyhow::Error;

        fn try_from(x: HuffmanCodeWord) -> Result<Self> {
            Ok(Self(x.0))
        }
    }

    #[test]
    fn from_lengths() -> Result<()> {
        let code = HuffmanCoding::<Value>::from_lengths(&[2, 3, 4, 3, 3, 4, 2])?;

        assert_eq!(
            code.decode_symbol(BitSequence::new(0b00, 2)),
            Some(Value(0)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b100, 3)),
            Some(Value(1)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b1110, 4)),
            Some(Value(2)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b101, 3)),
            Some(Value(3)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b110, 3)),
            Some(Value(4)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b1111, 4)),
            Some(Value(5)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b01, 2)),
            Some(Value(6)),
        );

        assert_eq!(code.decode_symbol(BitSequence::new(0b0, 1)), None);
        assert_eq!(code.decode_symbol(BitSequence::new(0b10, 2)), None);
        assert_eq!(code.decode_symbol(BitSequence::new(0b111, 3)), None,);

        Ok(())
    }

    #[test]
    fn read_symbol() -> Result<()> {
        let code = HuffmanCoding::<Value>::from_lengths(&[2, 3, 4, 3, 3, 4, 2])?;
        let mut data: &[u8] = &[0b10111001, 0b11001010, 0b11101101];
        let mut reader = BitReader::new(&mut data);

        assert_eq!(code.read_symbol(&mut reader)?, Value(1));
        assert_eq!(code.read_symbol(&mut reader)?, Value(2));
        assert_eq!(code.read_symbol(&mut reader)?, Value(3));
        assert_eq!(code.read_symbol(&mut reader)?, Value(6));
        assert_eq!(code.read_symbol(&mut reader)?, Value(0));
        assert_eq!(code.read_symbol(&mut reader)?, Value(2));
        assert_eq!(code.read_symbol(&mut reader)?, Value(4));
        assert!(code.read_symbol(&mut reader).is_err());

        Ok(())
    }

    #[test]
    fn from_lengths_with_zeros() -> Result<()> {
        let lengths = [3, 4, 5, 5, 0, 0, 6, 6, 4, 0, 6, 0, 7];
        let code = HuffmanCoding::<Value>::from_lengths(&lengths)?;
        let mut data: &[u8] = &[
            0b00100000, 0b00100001, 0b00010101, 0b10010101, 0b00110101, 0b00011101,
        ];
        let mut reader = BitReader::new(&mut data);

        assert_eq!(code.read_symbol(&mut reader)?, Value(0));
        assert_eq!(code.read_symbol(&mut reader)?, Value(1));
        assert_eq!(code.read_symbol(&mut reader)?, Value(2));
        assert_eq!(code.read_symbol(&mut reader)?, Value(3));
        assert_eq!(code.read_symbol(&mut reader)?, Value(6));
        assert_eq!(code.read_symbol(&mut reader)?, Value(7));
        assert_eq!(code.read_symbol(&mut reader)?, Value(8));
        assert_eq!(code.read_symbol(&mut reader)?, Value(10));
        assert_eq!(code.read_symbol(&mut reader)?, Value(12));
        assert!(code.read_symbol(&mut reader).is_err());

        Ok(())
    }

    #[test]
    fn from_lengths_additional() -> Result<()> {
        let lengths = [
            9, 10, 10, 8, 8, 8, 5, 6, 4, 5, 4, 5, 4, 5, 4, 4, 5, 4, 4, 5, 4, 5, 4, 5, 5, 5, 4, 6, 6,
        ];
        let code = HuffmanCoding::<Value>::from_lengths(&lengths)?;
        let mut data: &[u8] = &[
            0b11111000, 0b10111100, 0b01010001, 0b11111111, 0b00110101, 0b11111001, 0b11011111,
            0b11100001, 0b01110111, 0b10011111, 0b10111111, 0b00110100, 0b10111010, 0b11111111,
            0b11111101, 0b10010100, 0b11001110, 0b01000011, 0b11100111, 0b00000010,
        ];
        let mut reader = BitReader::new(&mut data);

        assert_eq!(code.read_symbol(&mut reader)?, Value(10));
        assert_eq!(code.read_symbol(&mut reader)?, Value(7));
        assert_eq!(code.read_symbol(&mut reader)?, Value(27));
        assert_eq!(code.read_symbol(&mut reader)?, Value(22));
        assert_eq!(code.read_symbol(&mut reader)?, Value(9));
        assert_eq!(code.read_symbol(&mut reader)?, Value(0));
        assert_eq!(code.read_symbol(&mut reader)?, Value(11));
        assert_eq!(code.read_symbol(&mut reader)?, Value(15));
        assert_eq!(code.read_symbol(&mut reader)?, Value(2));
        assert_eq!(code.read_symbol(&mut reader)?, Value(20));
        assert_eq!(code.read_symbol(&mut reader)?, Value(8));
        assert_eq!(code.read_symbol(&mut reader)?, Value(4));
        assert_eq!(code.read_symbol(&mut reader)?, Value(23));
        assert_eq!(code.read_symbol(&mut reader)?, Value(24));
        assert_eq!(code.read_symbol(&mut reader)?, Value(5));
        assert_eq!(code.read_symbol(&mut reader)?, Value(26));
        assert_eq!(code.read_symbol(&mut reader)?, Value(18));
        assert_eq!(code.read_symbol(&mut reader)?, Value(12));
        assert_eq!(code.read_symbol(&mut reader)?, Value(25));
        assert_eq!(code.read_symbol(&mut reader)?, Value(1));
        assert_eq!(code.read_symbol(&mut reader)?, Value(3));
        assert_eq!(code.read_symbol(&mut reader)?, Value(6));
        assert_eq!(code.read_symbol(&mut reader)?, Value(13));
        assert_eq!(code.read_symbol(&mut reader)?, Value(14));
        assert_eq!(code.read_symbol(&mut reader)?, Value(16));
        assert_eq!(code.read_symbol(&mut reader)?, Value(17));
        assert_eq!(code.read_symbol(&mut reader)?, Value(19));
        assert_eq!(code.read_symbol(&mut reader)?, Value(21));

        Ok(())
    }
}
