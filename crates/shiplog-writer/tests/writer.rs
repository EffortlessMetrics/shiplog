use shiplog_writer::{BufferedWriter, CountingWriter, LineWriter};
use std::fs;
use std::io::{Cursor, Write};
use tempfile::TempDir;

// ── BufferedWriter ────────────────────────────────────────────────

#[test]
fn buffered_writer_small_write_stays_buffered() {
    let cursor = Cursor::new(Vec::new());
    let mut w = BufferedWriter::new(cursor, 100);
    w.write(b"hi").unwrap();
    // Not flushed yet — inner should be empty
    assert!(w.get_ref().get_ref().is_empty());
    w.flush().unwrap();
    assert_eq!(w.get_ref().get_ref().as_slice(), b"hi");
}

#[test]
fn buffered_writer_large_write_goes_directly() {
    let cursor = Cursor::new(Vec::new());
    let mut w = BufferedWriter::new(cursor, 4);
    w.write(b"hello world").unwrap();
    // Larger than capacity, written directly
    assert_eq!(w.get_ref().get_ref().as_slice(), b"hello world");
}

#[test]
fn buffered_writer_flush_on_capacity() {
    let cursor = Cursor::new(Vec::new());
    let mut w = BufferedWriter::new(cursor, 5);
    w.write(b"ab").unwrap();
    // buffer now has 2 bytes
    w.write(b"cde").unwrap();
    // 2+3=5 >= capacity(5), should flush then buffer "cde"
    w.flush().unwrap();
    let output = w.get_ref().get_ref().clone();
    assert_eq!(output, b"abcde");
}

#[test]
fn buffered_writer_write_str() {
    let cursor = Cursor::new(Vec::new());
    let mut w = BufferedWriter::new(cursor, 100);
    w.write_str("hello 世界").unwrap();
    w.flush().unwrap();
    assert_eq!(
        String::from_utf8(w.get_ref().get_ref().clone()).unwrap(),
        "hello 世界"
    );
}

#[test]
fn buffered_writer_empty_write() {
    let cursor = Cursor::new(Vec::new());
    let mut w = BufferedWriter::new(cursor, 10);
    let n = w.write(b"").unwrap();
    assert_eq!(n, 0);
    w.flush().unwrap();
    assert!(w.get_ref().get_ref().is_empty());
}

#[test]
fn buffered_writer_multiple_flushes() {
    let cursor = Cursor::new(Vec::new());
    let mut w = BufferedWriter::new(cursor, 100);
    w.write(b"a").unwrap();
    w.flush().unwrap();
    w.write(b"b").unwrap();
    w.flush().unwrap();
    assert_eq!(w.get_ref().get_ref().as_slice(), b"ab");
}

#[test]
fn buffered_writer_get_mut() {
    let cursor = Cursor::new(Vec::new());
    let mut w = BufferedWriter::new(cursor, 10);
    w.write(b"test").unwrap();
    w.flush().unwrap();
    let inner = w.get_mut();
    assert_eq!(inner.get_ref(), b"test");
}

#[test]
fn buffered_writer_to_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("out.txt");
    let file = fs::File::create(&path).unwrap();
    let mut w = BufferedWriter::new(file, 64);
    w.write(b"file content").unwrap();
    w.flush().unwrap();
    drop(w);
    assert_eq!(fs::read_to_string(&path).unwrap(), "file content");
}

// ── LineWriter ────────────────────────────────────────────────────

#[test]
fn line_writer_single_line() {
    let cursor = Cursor::new(Vec::new());
    let mut w = LineWriter::new(cursor);
    w.write_line("hello").unwrap();
    assert_eq!(w.get_ref().get_ref().as_slice(), b"hello\n");
}

#[test]
fn line_writer_multiple_lines() {
    let cursor = Cursor::new(Vec::new());
    let mut w = LineWriter::new(cursor);
    w.write_line("one").unwrap();
    w.write_line("two").unwrap();
    w.write_line("three").unwrap();
    assert_eq!(w.get_ref().get_ref().as_slice(), b"one\ntwo\nthree\n");
}

#[test]
fn line_writer_empty_line() {
    let cursor = Cursor::new(Vec::new());
    let mut w = LineWriter::new(cursor);
    let n = w.write_line("").unwrap();
    assert_eq!(n, 1); // just the newline
    assert_eq!(w.get_ref().get_ref().as_slice(), b"\n");
}

#[test]
fn line_writer_unicode_lines() {
    let cursor = Cursor::new(Vec::new());
    let mut w = LineWriter::new(cursor);
    w.write_line("日本語テスト").unwrap();
    let s = String::from_utf8(w.get_ref().get_ref().clone()).unwrap();
    assert_eq!(s, "日本語テスト\n");
}

#[test]
fn line_writer_special_chars() {
    let cursor = Cursor::new(Vec::new());
    let mut w = LineWriter::new(cursor);
    w.write_line("tabs\there").unwrap();
    w.write_line("newline\ninside").unwrap();
    let s = String::from_utf8(w.get_ref().get_ref().clone()).unwrap();
    assert!(s.contains("tabs\there\n"));
}

#[test]
fn line_writer_implements_write_trait() {
    let cursor = Cursor::new(Vec::new());
    let mut w = LineWriter::new(cursor);
    // Use Write trait directly
    w.write_all(b"raw bytes").unwrap();
    w.flush().unwrap();
    assert_eq!(w.get_ref().get_ref().as_slice(), b"raw bytes");
}

#[test]
fn line_writer_to_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("lines.txt");
    let file = fs::File::create(&path).unwrap();
    let mut w = LineWriter::new(file);
    w.write_line("line1").unwrap();
    w.write_line("line2").unwrap();
    drop(w);
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, "line1\nline2\n");
}

// ── CountingWriter ────────────────────────────────────────────────

#[test]
fn counting_writer_initial_zero() {
    let cursor = Cursor::new(Vec::new());
    let w = CountingWriter::new(cursor);
    assert_eq!(w.bytes_written(), 0);
}

#[test]
fn counting_writer_tracks_bytes() {
    let cursor = Cursor::new(Vec::new());
    let mut w = CountingWriter::new(cursor);
    w.write_all(b"hello").unwrap();
    assert_eq!(w.bytes_written(), 5);
    w.write_all(b" world!").unwrap();
    assert_eq!(w.bytes_written(), 12);
}

#[test]
fn counting_writer_empty_write() {
    let cursor = Cursor::new(Vec::new());
    let mut w = CountingWriter::new(cursor);
    w.write_all(b"").unwrap();
    assert_eq!(w.bytes_written(), 0);
}

#[test]
fn counting_writer_content_correct() {
    let cursor = Cursor::new(Vec::new());
    let mut w = CountingWriter::new(cursor);
    w.write_all(b"data").unwrap();
    assert_eq!(w.get_ref().get_ref().as_slice(), b"data");
}

#[test]
fn counting_writer_flush() {
    let cursor = Cursor::new(Vec::new());
    let mut w = CountingWriter::new(cursor);
    w.write_all(b"test").unwrap();
    w.flush().unwrap();
    assert_eq!(w.bytes_written(), 4);
}

#[test]
fn counting_writer_large_data() {
    let cursor = Cursor::new(Vec::new());
    let mut w = CountingWriter::new(cursor);
    let data = vec![0xABu8; 10_000];
    w.write_all(&data).unwrap();
    assert_eq!(w.bytes_written(), 10_000);
}

#[test]
fn counting_writer_to_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("counted.bin");
    let file = fs::File::create(&path).unwrap();
    let mut w = CountingWriter::new(file);
    w.write_all(b"binary data").unwrap();
    assert_eq!(w.bytes_written(), 11);
    drop(w);
    assert_eq!(fs::read(&path).unwrap(), b"binary data");
}

// ── Property tests ────────────────────────────────────────────────

mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn counting_writer_byte_count_matches(data in prop::collection::vec(any::<u8>(), 0..1000)) {
            let cursor = Cursor::new(Vec::new());
            let mut w = CountingWriter::new(cursor);
            w.write_all(&data).unwrap();
            prop_assert_eq!(w.bytes_written(), data.len() as u64);
        }

        #[test]
        fn buffered_writer_roundtrip(data in prop::collection::vec(any::<u8>(), 0..500)) {
            let cursor = Cursor::new(Vec::new());
            let mut w = BufferedWriter::new(cursor, 64);
            w.write(&data).unwrap();
            w.flush().unwrap();
            prop_assert_eq!(w.get_ref().get_ref().as_slice(), data.as_slice());
        }

        #[test]
        fn line_writer_appends_newline(line in "\\PC{0,100}") {
            let cursor = Cursor::new(Vec::new());
            let mut w = LineWriter::new(cursor);
            w.write_line(&line).unwrap();
            let out = w.get_ref().get_ref().clone();
            prop_assert!(out.ends_with(b"\n"));
            let expected = format!("{}\n", line);
            prop_assert_eq!(out, expected.as_bytes().to_vec());
        }

        #[test]
        fn counting_writer_file_roundtrip(data in prop::collection::vec(any::<u8>(), 0..500)) {
            let tmp = TempDir::new().unwrap();
            let path = tmp.path().join("prop.bin");
            let file = fs::File::create(&path).unwrap();
            let mut w = CountingWriter::new(file);
            w.write_all(&data).unwrap();
            w.flush().unwrap();
            drop(w);
            let read_back = fs::read(&path).unwrap();
            prop_assert_eq!(read_back, data);
        }
    }
}
