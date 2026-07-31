//! Input cursor for the deserializer.
//!
//! A raw-pointer cursor over the input slice: every read is a single bounds
//! check against `end` followed by an unaligned load, and `read_slice`
//! returns a `&'de [u8]` borrowed straight from the input for zero-copy
//! strings and byte buffers.

use core::marker::PhantomData;

use super::error::{Error, Result};
use super::varint;

pub struct Reader<'de> {
    cursor: *const u8,
    end: *const u8,
    _marker: PhantomData<&'de [u8]>,
}

impl<'de> Reader<'de> {
    pub const fn new(buf: &'de [u8]) -> Self {
        let start = buf.as_ptr();
        Self {
            cursor: start,
            // SAFETY: `end` is the one-past-the-end pointer of `buf`.
            end: unsafe { start.add(buf.len()) },
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    pub fn remaining(&self) -> usize {
        (self.end as usize) - (self.cursor as usize)
    }

    #[inline(always)]
    pub fn read_byte(&mut self) -> Result<u8> {
        if self.cursor == self.end {
            return Err(Error::UnexpectedEnd);
        }
        // SAFETY: cursor < end, so one byte is readable.
        let b = unsafe { self.cursor.read() };
        // SAFETY: advancing by one stays within the buffer.
        self.cursor = unsafe { self.cursor.add(1) };
        Ok(b)
    }

    #[inline(always)]
    pub fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        if self.remaining() < N {
            return Err(Error::UnexpectedEnd);
        }
        // SAFETY: at least N bytes are readable at cursor.
        let a = unsafe { self.cursor.cast::<[u8; N]>().read_unaligned() };
        // SAFETY: advancing by N stays within the buffer.
        self.cursor = unsafe { self.cursor.add(N) };
        Ok(a)
    }

    #[inline(always)]
    pub fn read_slice(&mut self, n: usize) -> Result<&'de [u8]> {
        if self.remaining() < n {
            return Err(Error::UnexpectedEnd);
        }
        // SAFETY: at least n bytes are readable at cursor, and the buffer
        // outlives 'de.
        let s = unsafe { core::slice::from_raw_parts(self.cursor, n) };
        // SAFETY: advancing by n stays within the buffer.
        self.cursor = unsafe { self.cursor.add(n) };
        Ok(s)
    }

    #[inline(always)]
    pub fn read_varint_u16(&mut self) -> Result<u16> {
        let b = self.read_byte()?;
        if b <= varint::SINGLE_BYTE_MAX {
            return Ok(u16::from(b));
        }
        if b == varint::U16_TAG {
            return Ok(u16::from_le_bytes(self.read_array::<2>()?));
        }
        Err(Error::InvalidVarint)
    }

    #[inline(always)]
    pub fn read_varint_u32(&mut self) -> Result<u32> {
        let b = self.read_byte()?;
        if b <= varint::SINGLE_BYTE_MAX {
            return Ok(u32::from(b));
        }
        match b {
            varint::U16_TAG => Ok(u32::from(u16::from_le_bytes(self.read_array::<2>()?))),
            varint::U32_TAG => Ok(u32::from_le_bytes(self.read_array::<4>()?)),
            _ => Err(Error::InvalidVarint),
        }
    }

    #[inline(always)]
    pub fn read_varint_u64(&mut self) -> Result<u64> {
        let b = self.read_byte()?;
        if b <= varint::SINGLE_BYTE_MAX {
            return Ok(u64::from(b));
        }
        match b {
            varint::U16_TAG => Ok(u64::from(u16::from_le_bytes(self.read_array::<2>()?))),
            varint::U32_TAG => Ok(u64::from(u32::from_le_bytes(self.read_array::<4>()?))),
            varint::U64_TAG => Ok(u64::from_le_bytes(self.read_array::<8>()?)),
            _ => Err(Error::InvalidVarint),
        }
    }

    #[inline(always)]
    pub fn read_varint_u128(&mut self) -> Result<u128> {
        let b = self.read_byte()?;
        if b <= varint::SINGLE_BYTE_MAX {
            return Ok(u128::from(b));
        }
        match b {
            varint::U16_TAG => Ok(u128::from(u16::from_le_bytes(self.read_array::<2>()?))),
            varint::U32_TAG => Ok(u128::from(u32::from_le_bytes(self.read_array::<4>()?))),
            varint::U64_TAG => Ok(u128::from(u64::from_le_bytes(self.read_array::<8>()?))),
            varint::U128_TAG => Ok(u128::from_le_bytes(self.read_array::<16>()?)),
            _ => Err(Error::InvalidVarint),
        }
    }
}
