//! Output sinks for the serializer.
//!
//! The [`Writer`] abstraction uses reserve/commit cursor semantics: the
//! caller asks for `n` spare bytes, writes into the returned raw pointer,
//! then commits the number of bytes actually written. This lets varints be
//! written directly into the output buffer with a single capacity check.

use super::error::{Error, Result};

/// Maximum size of a direct `reserve` + pointer-write + `commit` write
/// (varints, short string prefixes). All larger or bulk writes go through
/// [`Writer::write_all`], which implementations may handle more efficiently.
pub const MAX_DIRECT_WRITE: usize = 256;

/// A sink for serialized bytes.
///
/// # Safety
/// Implementors must guarantee that after a successful `reserve(n)`, at
/// least `n` bytes are writable at the returned pointer, and that those
/// bytes remain stable until the matching `commit`.
///
/// # Contract
/// Direct `reserve` + pointer-write + `commit` is only used for small writes
/// of at most [`MAX_DIRECT_WRITE`] bytes; larger writes must go through
/// [`Writer::write_all`].
pub unsafe trait Writer {
    /// Ensures at least `n` spare bytes and returns the write cursor.
    fn reserve(&mut self, n: usize) -> Result<*mut u8>;

    /// Advances the write cursor past `n` bytes written via the pointer
    /// returned by the previous `reserve`.
    ///
    /// # Safety
    /// `n` must not exceed the size requested in the previous `reserve`,
    /// and those `n` bytes must have been initialized.
    unsafe fn commit(&mut self, n: usize);

    #[inline(always)]
    fn write_all(&mut self, bytes: &[u8]) -> Result<()> {
        let p = self.reserve(bytes.len())?;
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
            self.commit(bytes.len());
        }
        Ok(())
    }

    #[inline(always)]
    fn write_byte(&mut self, b: u8) -> Result<()> {
        let p = self.reserve(1)?;
        unsafe {
            p.write(b);
            self.commit(1);
        }
        Ok(())
    }
}

/// Writes into a caller-provided slice using raw pointer cursors.
pub struct SliceWriter<'a> {
    start: *mut u8,
    cursor: *mut u8,
    end: *mut u8,
    _marker: core::marker::PhantomData<&'a mut [u8]>,
}

impl<'a> SliceWriter<'a> {
    pub const fn new(buf: &'a mut [u8]) -> Self {
        let start = buf.as_mut_ptr();
        Self {
            start,
            cursor: start,
            // SAFETY: `end` is the one-past-the-end pointer of `buf`.
            end: unsafe { start.add(buf.len()) },
            _marker: core::marker::PhantomData,
        }
    }

    pub fn written(&self) -> usize {
        self.cursor as usize - self.start as usize
    }
}

// SAFETY: `reserve` returns `cursor` only after checking `cursor + n <= end`,
// so `n` bytes are writable; `cursor`/`end` stay within the original slice.
unsafe impl Writer for SliceWriter<'_> {
    #[inline(always)]
    fn reserve(&mut self, n: usize) -> Result<*mut u8> {
        if (self.end as usize) - (self.cursor as usize) < n {
            return Err(Error::ExceedsBuffer);
        }
        Ok(self.cursor)
    }

    #[inline(always)]
    unsafe fn commit(&mut self, n: usize) {
        // SAFETY: upheld by the caller contract of `commit`.
        self.cursor = unsafe { self.cursor.add(n) };
    }
}

/// Writes into the spare capacity of a `Vec<u8>`, growing on demand.
#[cfg(feature = "alloc")]
pub struct VecWriter<'a> {
    vec: &'a mut alloc::vec::Vec<u8>,
}

#[cfg(feature = "alloc")]
impl<'a> VecWriter<'a> {
    pub const fn new(vec: &'a mut alloc::vec::Vec<u8>) -> Self {
        Self { vec }
    }

    #[cold]
    #[inline(never)]
    fn grow(&mut self, n: usize) {
        self.vec.reserve(n);
    }
}

// SAFETY: `reserve` grows the Vec so that `n` spare bytes exist and returns a
// pointer into its spare capacity; the Vec is not touched between `reserve`
// and `commit`, so the pointer stays valid.
#[cfg(feature = "alloc")]
unsafe impl Writer for VecWriter<'_> {
    #[inline(always)]
    fn reserve(&mut self, n: usize) -> Result<*mut u8> {
        if self.vec.capacity() - self.vec.len() < n {
            self.grow(n);
        }
        // SAFETY: `len` is within the allocation, which has at least `n`
        // spare bytes at this point.
        Ok(unsafe { self.vec.as_mut_ptr().add(self.vec.len()) })
    }

    #[inline(always)]
    unsafe fn commit(&mut self, n: usize) {
        let len = self.vec.len();
        // SAFETY: the previous `reserve` guaranteed `n` initialized spare
        // bytes, which the caller wrote.
        unsafe { self.vec.set_len(len + n) };
    }
}

/// Counts the serialized size without storing anything.
///
/// `reserve` hands out a small scratch buffer (only direct writes, at most
/// [`MAX_DIRECT_WRITE`] bytes per the [`Writer`] contract); bulk data goes
/// through `write_all`, which is overridden to only count.
pub struct SizeWriter {
    pub size: usize,
    scratch: [u8; MAX_DIRECT_WRITE],
}

impl SizeWriter {
    pub const fn new() -> Self {
        Self {
            size: 0,
            scratch: [0; MAX_DIRECT_WRITE],
        }
    }
}

// SAFETY: all `reserve` calls in the serializer request at most
// `MAX_DIRECT_WRITE` bytes (per the `Writer` contract); the scratch buffer
// is that large.
unsafe impl Writer for SizeWriter {
    #[inline(always)]
    fn reserve(&mut self, n: usize) -> Result<*mut u8> {
        debug_assert!(n <= MAX_DIRECT_WRITE);
        Ok(self.scratch.as_mut_ptr())
    }

    #[inline(always)]
    unsafe fn commit(&mut self, n: usize) {
        self.size += n;
    }

    #[inline(always)]
    fn write_all(&mut self, bytes: &[u8]) -> Result<()> {
        self.size += bytes.len();
        Ok(())
    }

    #[inline(always)]
    fn write_byte(&mut self, _b: u8) -> Result<()> {
        self.size += 1;
        Ok(())
    }
}
