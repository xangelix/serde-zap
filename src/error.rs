use core::fmt;

/// Errors that can occur during serialization or deserialization.
///
/// Deliberately a small, fieldless enum: it is niche-optimized so that
/// `Result<T, Error>` stays cheap to return through every hot path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The output buffer is too small to hold the serialized data.
    ExceedsBuffer,
    /// The input ended before a complete value could be decoded.
    UnexpectedEnd,
    /// A varint tag byte was invalid for the target integer type.
    InvalidVarint,
    /// A boolean byte was not 0 or 1.
    InvalidBool,
    /// An option tag byte was not 0 or 1.
    InvalidOption,
    /// String data was not valid UTF-8.
    InvalidUtf8,
    /// A `u32` was not a valid unicode scalar value.
    InvalidChar,
    /// A length prefix did not fit in `usize`.
    InvalidLength,
    /// A sequence or map was serialized without a known length.
    SeqLengthUnknown,
    /// The format is not self-describing; this serde operation is unsupported.
    NotSupported,
    /// A custom serde error message (discarded).
    Custom,
}

pub type Result<T> = core::result::Result<T, Error>;

impl serde::ser::Error for Error {
    #[cold]
    #[inline(never)]
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        Self::Custom
    }
}

impl serde::de::Error for Error {
    #[cold]
    #[inline(never)]
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        Self::Custom
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::ExceedsBuffer => "output buffer too small",
            Self::UnexpectedEnd => "unexpected end of input",
            Self::InvalidVarint => "invalid varint tag",
            Self::InvalidBool => "invalid bool value",
            Self::InvalidOption => "invalid option tag",
            Self::InvalidUtf8 => "invalid UTF-8",
            Self::InvalidChar => "invalid char value",
            Self::InvalidLength => "length does not fit in usize",
            Self::SeqLengthUnknown => "sequence length must be known",
            Self::NotSupported => "operation not supported by this format",
            Self::Custom => "custom error",
        };
        f.write_str(msg)
    }
}

impl core::error::Error for Error {}
