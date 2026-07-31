//! The serde `Deserializer` implementation.

use serde::de::{self, DeserializeSeed, IntoDeserializer, Visitor};

use super::error::{Error, Result};
use super::read::Reader;

pub struct Deserializer<'de> {
    pub r: Reader<'de>,
}

impl<'de> Deserializer<'de> {
    pub const fn new(buf: &'de [u8]) -> Self {
        Self {
            r: Reader::new(buf),
        }
    }

    #[inline(always)]
    fn take_len(&mut self) -> Result<usize> {
        let n = self.r.read_varint_u64()?;
        usize::try_from(n).map_err(|_| Error::InvalidLength)
    }

    #[inline(always)]
    fn take_borrowed_str(&mut self) -> Result<&'de str> {
        let len = self.take_len()?;
        let bytes = self.r.read_slice(len)?;
        core::str::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)
    }
}

impl<'de> de::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::NotSupported)
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::NotSupported)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::NotSupported)
    }

    #[inline(always)]
    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.r.read_byte()? {
            0 => visitor.visit_bool(false),
            1 => visitor.visit_bool(true),
            _ => Err(Error::InvalidBool),
        }
    }

    #[inline(always)]
    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i8(self.r.read_byte()?.cast_signed())
    }

    #[inline(always)]
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i16(crate::varint::unzigzag_i16(self.r.read_varint_u16()?))
    }

    #[inline(always)]
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i32(crate::varint::unzigzag_i32(self.r.read_varint_u32()?))
    }

    #[inline(always)]
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i64(crate::varint::unzigzag_i64(self.r.read_varint_u64()?))
    }

    #[inline(always)]
    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i128(crate::varint::unzigzag_i128(self.r.read_varint_u128()?))
    }

    #[inline(always)]
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u8(self.r.read_byte()?)
    }

    #[inline(always)]
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u16(self.r.read_varint_u16()?)
    }

    #[inline(always)]
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u32(self.r.read_varint_u32()?)
    }

    #[inline(always)]
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u64(self.r.read_varint_u64()?)
    }

    #[inline(always)]
    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u128(self.r.read_varint_u128()?)
    }

    #[inline(always)]
    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_f32(f32::from_le_bytes(self.r.read_array::<4>()?))
    }

    #[inline(always)]
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_f64(f64::from_le_bytes(self.r.read_array::<8>()?))
    }

    #[inline(always)]
    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let v = self.r.read_varint_u32()?;
        let c = char::from_u32(v).ok_or(Error::InvalidChar)?;
        visitor.visit_char(c)
    }

    #[inline(always)]
    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_borrowed_str(self.take_borrowed_str()?)
    }

    #[inline(always)]
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_borrowed_str(self.take_borrowed_str()?)
    }

    #[inline(always)]
    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.take_len()?;
        visitor.visit_borrowed_bytes(self.r.read_slice(len)?)
    }

    #[inline(always)]
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.take_len()?;
        visitor.visit_borrowed_bytes(self.r.read_slice(len)?)
    }

    #[inline(always)]
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.r.read_byte()? {
            0 => visitor.visit_none(),
            1 => visitor.visit_some(self),
            _ => Err(Error::InvalidOption),
        }
    }

    #[inline(always)]
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }

    #[inline(always)]
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_unit()
    }

    #[inline(always)]
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    #[inline(always)]
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.take_len()?;
        visitor.visit_seq(Seq {
            de: self,
            remaining: len,
        })
    }

    #[inline(always)]
    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(Seq {
            de: self,
            remaining: len,
        })
    }

    #[inline(always)]
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_tuple(len, visitor)
    }

    #[inline(always)]
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.take_len()?;
        visitor.visit_map(Map {
            de: self,
            remaining: len,
        })
    }

    #[inline(always)]
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_seq(Seq {
            de: self,
            remaining: fields.len(),
        })
    }

    #[inline(always)]
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        let variant = self.r.read_varint_u32()?;
        visitor.visit_enum(EnumAccess { de: self, variant })
    }
}

struct Seq<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    remaining: usize,
}

impl<'de> de::SeqAccess<'de> for Seq<'_, 'de> {
    type Error = Error;

    #[inline(always)]
    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        seed.deserialize(&mut *self.de).map(Some)
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        // DoS guard: if the claimed element count exceeds the remaining input bytes,
        // the input is necessarily corrupt for elements of non-zero size, so refuse to hint.
        
        // This keeps deserializers (including serde_zap::full_vec) from preallocating a huge buffer.
        // Zero-sized-element sequences merely lose their preallocation, which is still correct.
        if self.remaining > self.de.r.remaining() {
            None
        } else {
            Some(self.remaining)
        }
    }
}

struct Map<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    remaining: usize,
}

impl<'de> de::MapAccess<'de> for Map<'_, 'de> {
    type Error = Error;

    #[inline(always)]
    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.remaining == 0 {
            return Ok(None);
        }
        seed.deserialize(&mut *self.de).map(Some)
    }

    #[inline(always)]
    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        self.remaining -= 1;
        seed.deserialize(&mut *self.de)
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        // See Seq::size_hint.
        if self.remaining > self.de.r.remaining() {
            None
        } else {
            Some(self.remaining)
        }
    }
}

struct EnumAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    variant: u32,
}

impl<'de> de::EnumAccess<'de> for EnumAccess<'_, 'de> {
    type Error = Error;
    type Variant = Self;

    #[inline(always)]
    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        let v = seed.deserialize(self.variant.into_deserializer())?;
        Ok((v, self))
    }
}

impl<'de> de::VariantAccess<'de> for EnumAccess<'_, 'de> {
    type Error = Error;

    #[inline(always)]
    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        seed.deserialize(self.de)
    }

    #[inline(always)]
    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        de::Deserializer::deserialize_tuple(self.de, len, visitor)
    }

    #[inline(always)]
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        de::Deserializer::deserialize_tuple(self.de, fields.len(), visitor)
    }
}
