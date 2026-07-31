//! Tagged-prefix varint encoding
//!
//! * `v <= 250`        -> a single byte `v`
//! * `v <= u16::MAX`   -> `251` followed by a little-endian `u16`
//! * `v <= u32::MAX`   -> `252` followed by a little-endian `u32`
//! * `v <= u64::MAX`   -> `253` followed by a little-endian `u64`
//! * otherwise         -> `254` followed by a little-endian `u128`
//!
//! Decoding is a single branch on the first byte followed by one bulk
//! fixed-width read, unlike LEB128 which branches per byte.
//!
//! The encode side lives in `crate::ser` (it needs access to the `Writer`):
//! each width branch reserves exactly the bytes it writes, so exact-fit
//! output buffers work.

pub const SINGLE_BYTE_MAX: u8 = 250;
pub const U16_TAG: u8 = 251;
pub const U32_TAG: u8 = 252;
pub const U64_TAG: u8 = 253;
pub const U128_TAG: u8 = 254;

#[inline(always)]
pub const fn zigzag_i16(v: i16) -> u16 {
    ((v << 1) ^ (v >> 15)).cast_unsigned()
}

#[inline(always)]
pub const fn zigzag_i32(v: i32) -> u32 {
    ((v << 1) ^ (v >> 31)).cast_unsigned()
}

#[inline(always)]
pub const fn zigzag_i64(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)).cast_unsigned()
}

#[inline(always)]
pub const fn zigzag_i128(v: i128) -> u128 {
    ((v << 1) ^ (v >> 127)).cast_unsigned()
}

#[inline(always)]
pub const fn unzigzag_i16(v: u16) -> i16 {
    ((v >> 1).cast_signed()) ^ -((v & 1).cast_signed())
}

#[inline(always)]
pub const fn unzigzag_i32(v: u32) -> i32 {
    ((v >> 1).cast_signed()) ^ -((v & 1).cast_signed())
}

#[inline(always)]
pub const fn unzigzag_i64(v: u64) -> i64 {
    ((v >> 1).cast_signed()) ^ -((v & 1).cast_signed())
}

#[inline(always)]
pub const fn unzigzag_i128(v: u128) -> i128 {
    ((v >> 1).cast_signed()) ^ -((v & 1).cast_signed())
}
