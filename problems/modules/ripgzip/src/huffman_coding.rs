#![forbid(unsafe_code)]

use std::{collections::HashMap, convert::TryFrom, io::BufRead};

use anyhow::{anyhow, bail, ensure, Context, Result};

use crate::bit_reader::{BitReader, BitSequence};

////////////////////////////////////////////////////////////////////////////////

pub fn decode_litlen_distance_trees<T: BufRead>(
    bit_reader: &mut BitReader<T>,
) -> Result<(HuffmanCoding<LitLenToken>, HuffmanCoding<DistanceToken>)> {
    // See RFC 1951, section 3.2.7.
    // TODO: your code goes here.
    unimplemented!()
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
        // TODO: your code goes here.
        unimplemented!()
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
        // TODO: your code goes here.
        unimplemented!()
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
pub struct DistanceToken {
    pub base: u16,
    pub extra_bits: u8,
}

impl TryFrom<HuffmanCodeWord> for DistanceToken {
    type Error = anyhow::Error;

    fn try_from(value: HuffmanCodeWord) -> Result<Self> {
        // See RFC 1951, section 3.2.5.
        // TODO: your code goes here.
        unimplemented!()
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
            // println!("{:b} {}", bit_sequence.bits(), bit_sequence.len());
            if let Some(value) = self.decode_symbol(bit_sequence) {
                return Ok(value);
            }
        }
        bail!("no suitable ")
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
        println!("{:?}", next_code);
        let mut huffman_map = HashMap::<BitSequence, T>::new();
        for (idx, &len) in code_lengths.iter().enumerate() {
            if len != 0 {
                let bits_val = next_code[len as usize];
                let bit_seq = BitSequence::new(bits_val, len);
                let val_code_word = T::try_from(HuffmanCodeWord(idx as u16))?;

                // println!("-- {:b} {}", bits_val, len);
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
