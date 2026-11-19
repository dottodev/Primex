//! Bounds-checked little-endian byte reader.
//!
//! Every read that crosses the end of the buffer returns a typed `BadInput`
//! error with the exact offset, expected byte count, and actual remaining
//! count. No API here returns a zero-filled or guessed value on failure.

use crate::error::{PdxError, Result};

#[derive(Debug, Clone)]
pub struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize,
    base: u64,
}

impl<'a> ByteReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            base: 0,
        }
    }

    /// `base` is the file offset of `data[0]`. It makes every reported offset
    /// an absolute file offset even when reading a sub-slice.
    pub fn with_base(data: &'a [u8], base: u64) -> Self {
        Self { data, pos: 0, base }
    }

    pub fn pos(&self) -> u64 {
        self.base + self.pos as u64
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn seek(&mut self, absolute: u64) -> Result<()> {
        if absolute < self.base {
            return Err(PdxError::bad_input("seek before buffer base")
                .at(absolute)
                .expected(format!(">= {}", self.base), absolute.to_string()));
        }
        let relative = absolute - self.base;
        if relative > self.data.len() as u64 {
            return Err(PdxError::bad_input("seek past end of buffer")
                .at(absolute)
                .expected(
                    format!("<= {}", self.base + self.data.len() as u64),
                    absolute.to_string(),
                ));
        }
        self.pos = relative as usize;
        Ok(())
    }

    pub fn skip(&mut self, count: usize) -> Result<()> {
        self.ensure(count, "skip")?;
        self.pos += count;
        Ok(())
    }

    fn ensure(&self, count: usize, what: &str) -> Result<()> {
        if self.remaining() < count {
            Err(PdxError::bad_input(format!("truncated {what}"))
                .at(self.pos())
                .expected(
                    format!("{count} bytes"),
                    format!("{} bytes", self.remaining()),
                ))
        } else {
            Ok(())
        }
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        self.ensure(1, "u8")?;
        let value = self.data[self.pos];
        self.pos += 1;
        Ok(value)
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let bytes: [u8; 2] = self.read_bytes(2)?.try_into().unwrap();
        Ok(u16::from_le_bytes(bytes))
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        let bytes: [u8; 4] = self.read_bytes(4)?.try_into().unwrap();
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        let bytes: [u8; 8] = self.read_bytes(8)?.try_into().unwrap();
        Ok(u64::from_le_bytes(bytes))
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        Ok(self.read_u32()? as i32)
    }

    pub fn peek_u32(&self) -> Result<u32> {
        if self.remaining() < 4 {
            return Err(PdxError::bad_input("truncated peek u32")
                .at(self.pos())
                .expected("4 bytes", format!("{} bytes", self.remaining())));
        }
        Ok(u32::from_le_bytes(
            self.data[self.pos..self.pos + 4].try_into().unwrap(),
        ))
    }

    pub fn read_bytes(&mut self, count: usize) -> Result<&'a [u8]> {
        self.ensure(count, "bytes")?;
        let slice = &self.data[self.pos..self.pos + count];
        self.pos += count;
        Ok(slice)
    }

    pub fn sub_reader(&mut self, count: usize) -> Result<ByteReader<'a>> {
        let start = self.pos();
        let bytes = self.read_bytes(count)?;
        Ok(ByteReader::with_base(bytes, start))
    }

    pub fn read_cstring(&mut self) -> Result<&'a [u8]> {
        let start = self.pos;
        while self.pos < self.data.len() && self.data[self.pos] != 0 {
            self.pos += 1;
        }
        if self.pos == self.data.len() {
            return Err(PdxError::bad_input("unterminated cstring")
                .at(self.base + start as u64)
                .expected("NUL terminator", "end of buffer"));
        }
        let slice = &self.data[start..self.pos];
        self.pos += 1;
        Ok(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_le_scalars_and_tracks_offset() {
        let mut r = ByteReader::new(&[0x78, 0x56, 0xEF, 0xBE, 0xAD, 0xDE, 0x0A, 0x0B]);
        assert_eq!(r.read_u16().unwrap(), 0x5678);
        assert_eq!(r.pos(), 2);
        assert_eq!(r.read_u32().unwrap(), 0xDEADBEEF);
        assert_eq!(r.pos(), 6);
        assert_eq!(r.read_u8().unwrap(), 0x0A);
    }

    #[test]
    fn truncated_read_is_typed_with_offset() {
        let mut r = ByteReader::with_base(&[0x01, 0x02], 0x1000);
        let err = r.read_u32().unwrap_err();
        assert_eq!(err.kind, crate::error::ErrorKind::BadInput);
        assert_eq!(err.offset, Some(0x1000));
        assert_eq!(err.expected.as_deref(), Some("4 bytes"));
        assert_eq!(err.actual.as_deref(), Some("2 bytes"));
    }

    #[test]
    fn seek_out_of_range_is_typed() {
        let mut r = ByteReader::with_base(&[0; 4], 0x100);
        assert!(r.seek(0x104).is_ok());
        assert!(r.seek(0x105).is_err());
        assert!(r.seek(0xff).is_err());
    }

    #[test]
    fn sub_reader_keeps_absolute_base() {
        let mut r = ByteReader::with_base(&[1, 2, 3, 4], 0x200);
        r.skip(2).unwrap();
        let mut sub = r.sub_reader(2).unwrap();
        assert_eq!(sub.pos(), 0x202);
        assert_eq!(sub.read_u16().unwrap(), 0x0403);
    }

    #[test]
    fn unterminated_cstring_is_typed() {
        let mut r = ByteReader::new(b"abc");
        let err = r.read_cstring().unwrap_err();
        assert_eq!(err.offset, Some(0));
    }
}
