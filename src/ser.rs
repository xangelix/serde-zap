//! The serde `Serializer` implementation.

use serde::ser::{self, Serialize};

use super::error::{Error, Result};
use super::varint;
use super::write::Writer;

pub struct Ser<'a, W: Writer> {
    w: &'a mut W,
}

impl<'a, W: Writer> Ser<'a, W> {
    pub const fn new(w: &'a mut W) -> Self {
        Ser { w }
    }

    /// Writes a `u64`-range varint directly into the output. Each width
    /// branch reserves exactly the bytes it writes (exact-fit buffers work),
    /// and the common single-byte case is a plain `write_byte`.
    #[inline(always)]
    fn put_varint_u64(&mut self, v: u64) -> Result<()> {
        if v <= u64::from(varint::SINGLE_BYTE_MAX) {
            let [b, ..] = v.to_le_bytes();
            return self.w.write_byte(b);
        }
        if let Ok(v16) = u16::try_from(v) {
            let p = self.w.reserve(3)?;
            // SAFETY: reserve guaranteed 3 writable bytes.
            unsafe {
                p.write(varint::U16_TAG);
                p.add(1).cast::<u16>().write_unaligned(v16.to_le());
                self.w.commit(3);
            }
            return Ok(());
        }
        if let Ok(v32) = u32::try_from(v) {
            let p = self.w.reserve(5)?;
            // SAFETY: reserve guaranteed 5 writable bytes.
            unsafe {
                p.write(varint::U32_TAG);
                p.add(1).cast::<u32>().write_unaligned(v32.to_le());
                self.w.commit(5);
            }
            return Ok(());
        }
        let p = self.w.reserve(9)?;
        // SAFETY: reserve guaranteed 9 writable bytes.
        unsafe {
            p.write(varint::U64_TAG);
            p.add(1).cast::<u64>().write_unaligned(v.to_le());
            self.w.commit(9);
        }
        Ok(())
    }

    #[inline(always)]
    fn put_varint_u128(&mut self, v: u128) -> Result<()> {
        if let Ok(v64) = u64::try_from(v) {
            return self.put_varint_u64(v64);
        }
        let p = self.w.reserve(17)?;
        // SAFETY: reserve guaranteed 17 writable bytes.
        unsafe {
            p.write(varint::U128_TAG);
            p.add(1).cast::<u128>().write_unaligned(v.to_le());
            self.w.commit(17);
        }
        Ok(())
    }

    #[inline(always)]
    fn put_len(&mut self, len: usize) -> Result<()> {
        self.put_varint_u64(len as u64)
    }

    #[inline(always)]
    fn put_variant_index(&mut self, index: u32) -> Result<()> {
        self.put_varint_u64(u64::from(index))
    }

    /// Writes a length-prefixed byte string. The common short case
    /// (single-byte length prefix) costs a single reserve and one copy
    /// instead of two separate writes.
    #[inline(always)]
    fn put_len_prefixed_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let len = bytes.len();
        if len <= varint::SINGLE_BYTE_MAX as usize {
            let p = self.w.reserve(1 + len)?;
            // SAFETY: reserve guaranteed 1 + len writable bytes, and
            // 1 + len <= 1 + 250 <= MAX_DIRECT_WRITE (Writer contract).
            let [b, ..] = len.to_le_bytes();
            unsafe {
                p.write(b);
                core::ptr::copy_nonoverlapping(bytes.as_ptr(), p.add(1), len);
                self.w.commit(1 + len);
            }
            return Ok(());
        }
        self.put_len(len)?;
        self.w.write_all(bytes)
    }
}

impl<W: Writer> ser::Serializer for Ser<'_, W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    #[inline(always)]
    fn serialize_bool(self, v: bool) -> Result<()> {
        self.w.write_byte(u8::from(v))
    }

    #[inline(always)]
    fn serialize_i8(self, v: i8) -> Result<()> {
        self.w.write_byte(v.cast_unsigned())
    }

    #[inline(always)]
    fn serialize_i16(mut self, v: i16) -> Result<()> {
        self.put_varint_u64(u64::from(varint::zigzag_i16(v)))
    }

    #[inline(always)]
    fn serialize_i32(mut self, v: i32) -> Result<()> {
        self.put_varint_u64(u64::from(varint::zigzag_i32(v)))
    }

    #[inline(always)]
    fn serialize_i64(mut self, v: i64) -> Result<()> {
        self.put_varint_u64(varint::zigzag_i64(v))
    }

    #[inline(always)]
    fn serialize_i128(mut self, v: i128) -> Result<()> {
        self.put_varint_u128(varint::zigzag_i128(v))
    }

    #[inline(always)]
    fn serialize_u8(self, v: u8) -> Result<()> {
        self.w.write_byte(v)
    }

    #[inline(always)]
    fn serialize_u16(mut self, v: u16) -> Result<()> {
        self.put_varint_u64(u64::from(v))
    }

    #[inline(always)]
    fn serialize_u32(mut self, v: u32) -> Result<()> {
        self.put_varint_u64(u64::from(v))
    }

    #[inline(always)]
    fn serialize_u64(mut self, v: u64) -> Result<()> {
        self.put_varint_u64(v)
    }

    #[inline(always)]
    fn serialize_u128(mut self, v: u128) -> Result<()> {
        self.put_varint_u128(v)
    }

    #[inline(always)]
    fn serialize_f32(self, v: f32) -> Result<()> {
        self.w.write_all(&v.to_le_bytes())
    }

    #[inline(always)]
    fn serialize_f64(self, v: f64) -> Result<()> {
        self.w.write_all(&v.to_le_bytes())
    }

    #[inline(always)]
    fn serialize_char(mut self, v: char) -> Result<()> {
        self.put_varint_u64(u64::from(u32::from(v)))
    }

    #[inline(always)]
    fn serialize_str(mut self, v: &str) -> Result<()> {
        self.put_len_prefixed_bytes(v.as_bytes())
    }

    #[inline(always)]
    fn serialize_bytes(mut self, v: &[u8]) -> Result<()> {
        self.put_len_prefixed_bytes(v)
    }

    #[inline(always)]
    fn serialize_none(self) -> Result<()> {
        self.w.write_byte(0)
    }

    #[inline(always)]
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<()> {
        self.w.write_byte(1)?;
        value.serialize(self)
    }

    #[inline(always)]
    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn serialize_unit_variant(
        mut self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        self.put_variant_index(variant_index)
    }

    #[inline(always)]
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(self)
    }

    #[inline(always)]
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        mut self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<()> {
        self.put_variant_index(variant_index)?;
        value.serialize(self)
    }

    #[inline(always)]
    fn serialize_seq(mut self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        let len = len.ok_or(Error::SeqLengthUnknown)?;
        self.put_len(len)?;
        Ok(self)
    }

    #[inline(always)]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(self)
    }

    #[inline(always)]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(self)
    }

    #[inline(always)]
    fn serialize_tuple_variant(
        mut self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.put_variant_index(variant_index)?;
        Ok(self)
    }

    #[inline(always)]
    fn serialize_map(mut self, len: Option<usize>) -> Result<Self::SerializeMap> {
        let len = len.ok_or(Error::SeqLengthUnknown)?;
        self.put_len(len)?;
        Ok(self)
    }

    #[inline(always)]
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(self)
    }

    #[inline(always)]
    fn serialize_struct_variant(
        mut self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.put_variant_index(variant_index)?;
        Ok(self)
    }

    /// Without `alloc` there is no serde-provided default for `collect_str`
    /// (it formats into a `String`); fail cleanly instead.
    #[cfg(not(feature = "alloc"))]
    fn collect_str<T>(self, _value: &T) -> Result<()>
    where
        T: ?Sized + core::fmt::Display,
    {
        Err(Error::NotSupported)
    }
}

macro_rules! impl_compound {
    ($trait:ident, $method:ident) => {
        impl<W: Writer> ser::$trait for Ser<'_, W> {
            type Ok = ();
            type Error = Error;

            #[inline(always)]
            fn $method<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
                value.serialize(Ser { w: &mut *self.w })
            }

            #[inline(always)]
            fn end(self) -> Result<()> {
                Ok(())
            }
        }
    };
}

impl_compound!(SerializeSeq, serialize_element);
impl_compound!(SerializeTuple, serialize_element);
impl_compound!(SerializeTupleStruct, serialize_field);
impl_compound!(SerializeTupleVariant, serialize_field);

impl<W: Writer> ser::SerializeMap for Ser<'_, W> {
    type Ok = ();
    type Error = Error;

    #[inline(always)]
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        key.serialize(Ser { w: &mut *self.w })
    }

    #[inline(always)]
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(Ser { w: &mut *self.w })
    }

    #[inline(always)]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<W: Writer> ser::SerializeStruct for Ser<'_, W> {
    type Ok = ();
    type Error = Error;

    #[inline(always)]
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(Ser { w: &mut *self.w })
    }

    #[inline(always)]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<W: Writer> ser::SerializeStructVariant for Ser<'_, W> {
    type Ok = ();
    type Error = Error;

    #[inline(always)]
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(Ser { w: &mut *self.w })
    }

    #[inline(always)]
    fn end(self) -> Result<()> {
        Ok(())
    }
}
