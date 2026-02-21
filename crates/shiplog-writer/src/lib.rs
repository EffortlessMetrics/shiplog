//! Writer utilities for shiplog.
//!
//! This crate provides writer implementations for outputting data in various formats.

use std::io::{self, Write};

/// A buffered writer that batches writes for efficiency
pub struct BufferedWriter<W: Write> {
    inner: W,
    buffer: Vec<u8>,
    capacity: usize,
}

impl<W: Write> BufferedWriter<W> {
    pub fn new(inner: W, capacity: usize) -> Self {
        Self {
            inner,
            buffer: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Get a reference to the inner writer
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Get a mutable reference to the inner writer
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    pub fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.buffer.len() + data.len() >= self.capacity {
            self.flush()?;
        }
        
        if data.len() >= self.capacity {
            // Data is larger than buffer, write directly
            self.inner.write(data)?;
            Ok(data.len())
        } else {
            self.buffer.extend_from_slice(data);
            Ok(data.len())
        }
    }

    pub fn write_str(&mut self, s: &str) -> io::Result<usize> {
        self.write(s.as_bytes())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        if !self.buffer.is_empty() {
            self.inner.write_all(&self.buffer)?;
            self.buffer.clear();
        }
        self.inner.flush()
    }
}

impl<W: Write> Drop for BufferedWriter<W> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

/// A line writer that writes data line by line
pub struct LineWriter<W: Write> {
    inner: W,
}

impl<W: Write> LineWriter<W> {
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    /// Get a reference to the inner writer
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Get a mutable reference to the inner writer
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    pub fn write_line(&mut self, line: &str) -> io::Result<usize> {
        let mut bytes = line.as_bytes().to_vec();
        bytes.push(b'\n');
        self.inner.write_all(&bytes)?;
        Ok(bytes.len())
    }
}

impl<W: Write> Write for LineWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// A counting writer that tracks bytes written
pub struct CountingWriter<W: Write> {
    inner: W,
    bytes_written: u64,
}

impl<W: Write> CountingWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            bytes_written: 0,
        }
    }

    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// Get a reference to the inner writer
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Get a mutable reference to the inner writer
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.bytes_written += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_buffered_writer() {
        let cursor = Cursor::new(Vec::new());
        let mut writer = BufferedWriter::new(cursor, 4);

        writer.write(b"hello").unwrap();
        // Buffer should have 5 bytes, which exceeds capacity of 4
        // so it should have been flushed

        let cursor = writer.get_ref();
        assert_eq!(cursor.get_ref(), b"hello");
    }

    #[test]
    fn test_buffered_writer_flush() {
        let cursor = Cursor::new(Vec::new());
        let mut writer = BufferedWriter::new(cursor, 10);

        writer.write(b"hello").unwrap();
        writer.flush().unwrap();

        let cursor = writer.get_ref();
        assert_eq!(cursor.get_ref(), b"hello");
    }

    #[test]
    fn test_line_writer() {
        let cursor = Cursor::new(Vec::new());
        let mut writer = LineWriter::new(cursor);

        writer.write_line("hello").unwrap();
        writer.write_line("world").unwrap();

        let cursor = writer.get_ref();
        assert_eq!(cursor.get_ref(), b"hello\nworld\n");
    }

    #[test]
    fn test_counting_writer() {
        let cursor = Cursor::new(Vec::new());
        let mut writer = CountingWriter::new(cursor);

        writer.write(b"hello").unwrap();
        assert_eq!(writer.bytes_written(), 5);

        writer.write(b" world").unwrap();
        assert_eq!(writer.bytes_written(), 11);
    }

    #[test]
    fn test_counting_writer_inner() {
        let cursor = Cursor::new(Vec::new());
        let mut writer = CountingWriter::new(cursor);

        writer.write(b"test").unwrap();
        
        let cursor = writer.get_mut();
        assert_eq!(cursor.get_ref(), b"test");
    }
}
