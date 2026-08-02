//! `std::io` stream adapters (available with the `std` feature).
//!
//! [`to_writer`] serializes into any [`Write`] with an internal staging
//! buffer, so even unbuffered writers don't get one `write_all` call per
//! field. [`from_reader`] buffers a whole message from any [`Read`] and
//! deserializes it. Both preserve underlying I/O errors verbatim (the
//! functions return [`std::io::Result`], keeping `serde_zap::Error` small
//! for the in-memory API).
//!
//! ```
//! # use serde::{Serialize, Deserialize};
//! # #[derive(Serialize, Deserialize, PartialEq, Debug)]
//! # struct Reading { sensor_id: u32 }
//! # let reading = Reading { sensor_id: 7 };
//! // Any io::Write works; Vec<u8> here, but files, sockets, BufWriter, ...
//! let mut buf = Vec::new();
//! serde_zap::to_writer(&reading, &mut buf).unwrap();
//!
//! // Owned values only (the input buffer is dropped after the call).
//! let back: Reading = serde_zap::from_reader(&mut std::io::Cursor::new(&buf)).unwrap();
//! assert_eq!(back, reading);
//! ```

use alloc::vec::Vec;
use std::io::{Error as IoError, ErrorKind, Read, Write};

use serde::{Serialize, de::DeserializeOwned};

use super::error::Error;
use super::ser::Ser;
use super::write::{MAX_DIRECT_WRITE, Writer};

/// Size of the buffer [`IoWriter`] accumulates small writes in before
/// flushing to the underlying writer.
const STAGE_SIZE: usize = 8192;

struct IoWriter<'a, W: Write> {
    inner: &'a mut W,
    stage: [u8; STAGE_SIZE],
    stage_len: usize,
    /// The first underlying I/O error, kept verbatim for `to_writer` to
    /// return (the `Writer` trait itself can only report `Error`).
    io_err: Option<IoError>,
}

impl<'a, W: Write> IoWriter<'a, W> {
    const fn new(inner: &'a mut W) -> Self {
        Self {
            inner,
            stage: [0; STAGE_SIZE],
            stage_len: 0,
            io_err: None,
        }
    }

    fn flush(&mut self) -> Result<(), Error> {
        if self.stage_len > 0 && self.io_err.is_none() {
            // stage_len <= STAGE_SIZE by construction (reserve enforces it).
            let staged = self
                .stage
                .get(..self.stage_len)
                .ok_or(Error::NotSupported)?;
            if let Err(e) = self.inner.write_all(staged) {
                self.io_err = Some(e);
                return Err(Error::NotSupported);
            }
            self.stage_len = 0;
        }
        Ok(())
    }

    fn finish(mut self) -> std::io::Result<()> {
        match self.flush() {
            Ok(()) => Ok(()),
            Err(_) => Err(self
                .io_err
                .unwrap_or_else(|| IoError::other(Error::NotSupported))),
        }
    }
}

// SAFETY: direct reserves are capped at MAX_DIRECT_WRITE (256) bytes per the
// Writer contract; after a flush the stage always has room for them.
unsafe impl<W: Write> Writer for IoWriter<'_, W> {
    #[inline(always)]
    fn reserve(&mut self, n: usize) -> Result<*mut u8, Error> {
        debug_assert!(n <= MAX_DIRECT_WRITE);
        if self.stage_len + n > STAGE_SIZE {
            self.flush()?;
        }
        debug_assert!(self.stage_len + n <= STAGE_SIZE);
        // SAFETY: the stage has at least n free bytes at stage_len.
        Ok(unsafe { self.stage.as_mut_ptr().add(self.stage_len) })
    }

    #[inline(always)]
    unsafe fn commit(&mut self, n: usize) {
        self.stage_len += n;
    }

    #[inline(always)]
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let len = bytes.len();
        if len <= STAGE_SIZE - self.stage_len {
            // Fits in the stage: copy in and keep going (the common path —
            // varints, floats, and short strings all land here).
            // SAFETY: the stage has at least len free bytes at stage_len.
            unsafe {
                core::ptr::copy_nonoverlapping(
                    bytes.as_ptr(),
                    self.stage.as_mut_ptr().add(self.stage_len),
                    len,
                );
            }
            self.stage_len += len;
            return Ok(());
        }
        if len >= STAGE_SIZE / 2 {
            // Genuinely large write: flush, then write through to avoid a
            // pointless extra copy of the whole buffer.
            self.flush()?;
            if self.io_err.is_none()
                && let Err(e) = self.inner.write_all(bytes)
            {
                self.io_err = Some(e);
                return Err(Error::NotSupported);
            }
            return Ok(());
        }
        // Medium write that doesn't fit: flush, then stage it.
        self.flush()?;
        debug_assert_eq!(self.stage_len, 0);
        // SAFETY: len < STAGE_SIZE / 2, so the stage has room.
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), self.stage.as_mut_ptr(), len);
        }
        self.stage_len = len;
        Ok(())
    }
}

fn invalid_data(e: Error) -> IoError {
    IoError::new(ErrorKind::InvalidData, e)
}

/// Serializes `value` into `writer`.
///
/// Small writes (varints, short strings) accumulate in an 8 KiB staging
/// buffer that is flushed as needed and at the end; bulk writes go straight
/// to the underlying writer after a flush. Wrapping `writer` in a
/// [`std::io::BufWriter`] is allowed but not required.
///
/// # Errors
/// Returns the underlying I/O error verbatim, or an
/// [`ErrorKind::InvalidData`] wrapping a [`serde_zap::Error`] if the value
/// could not be serialized (e.g. a sequence without a known length).
///
/// [`serde_zap::Error`]: crate::Error
pub fn to_writer<T: Serialize + ?Sized, W: Write>(
    value: &T,
    writer: &mut W,
) -> std::io::Result<()> {
    let mut w = IoWriter::new(writer);
    match value.serialize(Ser::new(&mut w)) {
        Ok(()) => w.finish(),
        Err(e) => Err(w.io_err.unwrap_or_else(|| invalid_data(e))),
    }
}

/// Deserializes a `T` from `reader` by buffering the whole message first.
///
/// Owned values only: the input buffer is dropped after the call, so `T`
/// must be [`DeserializeOwned`] (no borrowed `&str`/`&[u8]` fields). For
/// zero-copy borrowing, use [`crate::from_bytes`] on a slice you own.
///
/// # Errors
/// Returns the underlying I/O error verbatim, or an
/// [`ErrorKind::InvalidData`] wrapping a [`serde_zap::Error`] if the input
/// is truncated or malformed.
///
/// [`serde_zap::Error`]: crate::Error
pub fn from_reader<T: DeserializeOwned, R: Read>(reader: &mut R) -> std::io::Result<T> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    crate::from_bytes(&buf).map_err(invalid_data)
}
