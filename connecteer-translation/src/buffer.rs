#[cfg(feature = "alloc")]
pub use with_std::*;

#[cfg(feature = "alloc")]
mod with_std {
    use std::ops::AddAssign;
    pub struct RingBuffer {
        current_bytes_read: usize,
        inner: std::collections::VecDeque<u8>,
    }

    impl RingBuffer {
        pub fn reset_read_bytes(&mut self) {
            self.current_bytes_read = 0;
        }

        pub fn discard_read_bytes(&mut self) {
            self.inner.drain(..self.current_bytes_read);
        }

        pub fn new(capacity: usize) -> Self {
            Self {
                current_bytes_read: 0,
                inner: std::collections::VecDeque::with_capacity(capacity),
            }
        }

        pub fn as_read(&mut self) -> BufferRead<'_> {
            BufferRead {
                bytes_read: &mut self.current_bytes_read,
                iter: self.inner.iter(),
            }
        }

        pub fn feed_bytes(&mut self, bytes: &[u8]) {
            self.inner.extend(bytes.iter());
        }
    }

    pub struct BufferRead<'buf> {
        bytes_read: &'buf mut usize,
        iter: std::collections::vec_deque::Iter<'buf, u8>,
    }

    impl embedded_io::Io for BufferRead<'_> {
        type Error = core::convert::Infallible;
    }

    impl embedded_io::blocking::Read for BufferRead<'_> {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            Ok(self
                .iter
                .by_ref()
                .take(buf.len())
                .zip(buf.iter_mut())
                .map(|(&r, w)| *w = r)
                .inspect(|()| self.bytes_read.add_assign(1))
                .count())
        }
    }

    unsafe impl super::Buffer for RingBuffer {
        type Reader<'a> = BufferRead<'a>;

        fn get_read(&mut self) -> Self::Reader<'_> {
            self.as_read()
        }

        fn feed_bytes(&mut self, bytes: &[u8]) -> usize {
            RingBuffer::feed_bytes(self, bytes);
            bytes.len()
        }

        fn keep_read_bytes(&mut self) {
            RingBuffer::reset_read_bytes(self)
        }

        fn discard_read_bytes(&mut self) {
            RingBuffer::reset_read_bytes(self)
        }
    }
}

pub unsafe trait Buffer {
    type Reader<'a>: embedded_io::blocking::Read<Error = core::convert::Infallible> + 'a
    where
        Self: 'a;

    /// Feed bytes into the Buffer
    /// The return value is the number of bytes that were copied into the buffer, so the caller
    /// knows which bytes to keep (if any)
    fn feed_bytes(&mut self, bytes: &[u8]) -> usize;
    /// Get a Reader into the Buffer
    /// This reader needs to NOT discard bytes while reading since there are dedicated methods for
    /// discarding these bytes (or resetting read head)
    fn get_read(&mut self) -> Self::Reader<'_>;
    /// Discard every bytes that were read from an `Self::Reader`
    fn discard_read_bytes(&mut self);
    /// Keep the bytes that were read into the buffer, allowing them to be re-read when a new
    /// Reader is reading into the buffer
    fn keep_read_bytes(&mut self);
}
