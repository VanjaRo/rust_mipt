#![forbid(unsafe_code)]

use std::io::{self, BufRead};

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BitSequence {
    bits: u16,
    len: u8,
}

impl BitSequence {
    pub fn new(bits: u16, len: u8) -> Self {
        assert!(len <= 16, "the length shouldn't exceed 16 bits");

        Self {
            bits: bits,
            len: len,
        }
    }

    pub fn bits(&self) -> u16 {
        self.bits
    }

    pub fn len(&self) -> u8 {
        self.len
    }

    pub fn concat(self, other: Self) -> Self {
        let total_len = self.len + other.len();
        assert!(total_len <= 16);

        Self {
            bits: other.bits() << self.len() | self.bits(),
            len: total_len,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct BitReader<T> {
    stream: T,
    reminder: BitSequence,
}

impl<T: BufRead> BitReader<T> {
    pub fn new(stream: T) -> Self {
        Self {
            stream,
            reminder: BitSequence::new(0, 0),
        }
    }

    pub fn read_bits(&mut self, len: u8) -> io::Result<BitSequence> {
        assert!(len <= 8);
        if self.reminder.len() < len {
            if let Some(fst_val) = self.stream.fill_buf()?.get(0) {
                self.reminder = self.reminder.concat(BitSequence::new(*fst_val as u16, 8));
                self.stream.consume(1);
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "can't read the requested amount of bits",
                ));
            }
        }
        let reminder_old = self.reminder;
        // Getting values of the "len" amount of bits
        let trailing_mask: u16 = (1 << len) - 1;
        let bits_to_ret = reminder_old.bits() & trailing_mask;
        self.reminder = BitSequence::new(reminder_old.bits() >> len, reminder_old.len() - len);

        Ok(BitSequence::new(bits_to_ret, len))
    }

    /// Discard all the unread bits in the current byte and return a mutable reference
    /// to the underlying reader.
    pub fn borrow_reader_from_boundary(&mut self) -> &mut T {
        self.reminder = BitSequence::new(0, 0);
        &mut self.stream
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::ReadBytesExt;

    #[test]
    fn read_bits() -> io::Result<()> {
        let data: &[u8] = &[0b01100011, 0b11011011, 0b10101111];
        let mut reader: BitReader<&[u8]> = BitReader::new(data);
        assert_eq!(reader.read_bits(1)?, BitSequence::new(0b1, 1));
        assert_eq!(reader.read_bits(2)?, BitSequence::new(0b01, 2));
        assert_eq!(reader.read_bits(3)?, BitSequence::new(0b100, 3));
        assert_eq!(reader.read_bits(4)?, BitSequence::new(0b1101, 4));
        assert_eq!(reader.read_bits(5)?, BitSequence::new(0b10110, 5));
        assert_eq!(reader.read_bits(8)?, BitSequence::new(0b01011111, 8));
        assert_eq!(
            reader.read_bits(2).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
        Ok(())
    }

    #[test]
    fn borrow_reader_from_boundary() -> io::Result<()> {
        let data: &[u8] = &[0b01100011, 0b11011011, 0b10101111];
        let mut reader = BitReader::new(data);
        assert_eq!(reader.read_bits(3)?, BitSequence::new(0b011, 3));
        assert_eq!(reader.borrow_reader_from_boundary().read_u8()?, 0b11011011);
        assert_eq!(reader.read_bits(8)?, BitSequence::new(0b10101111, 8));
        Ok(())
    }
}
