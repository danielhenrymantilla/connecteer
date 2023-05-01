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

impl std::io::Read for BufferRead<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(self
            .iter
            .by_ref()
            .zip(buf.iter_mut())
            .map(|(&r, w)| *w = r)
            .inspect(|()| self.bytes_read.add_assign(1))
            .count())
    }
}
