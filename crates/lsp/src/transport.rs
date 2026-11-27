//! JSON-RPC のフレーム（`Content-Length: N\r\n\r\n<body>`）。

use std::io::{self, BufRead, Write};

pub fn write_message(w: &mut impl Write, body: &[u8]) -> io::Result<()> {
    write!(w, "Content-Length: {}\r\n\r\n", body.len())?;
    w.write_all(body)?;
    w.flush()
}

/// 1つのメッセージを読む。ストリームが終わったら `Ok(None)`。
pub fn read_message(r: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let length = length.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Content-Length がない"))?;
    if length > 256 * 1024 * 1024 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "メッセージが大きすぎる"));
    }
    let mut body = vec![0; length];
    r.read_exact(&mut body)?;
    Ok(Some(body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_frames() {
        let mut buf = Vec::new();
        write_message(&mut buf, br#"{"a":1}"#).unwrap();
        write_message(&mut buf, "{\"t\":\"日本\"}".as_bytes()).unwrap();
        let mut r = io::Cursor::new(buf);
        assert_eq!(read_message(&mut r).unwrap().unwrap(), br#"{"a":1}"#);
        assert_eq!(
            String::from_utf8(read_message(&mut r).unwrap().unwrap()).unwrap(),
            "{\"t\":\"日本\"}"
        );
        assert!(read_message(&mut r).unwrap().is_none());
    }

    #[test]
    fn tolerates_extra_headers_and_rejects_missing_length() {
        let mut r = io::Cursor::new(b"Content-Type: x\r\ncontent-length: 2\r\n\r\n{}".to_vec());
        assert_eq!(read_message(&mut r).unwrap().unwrap(), b"{}");
        let mut r = io::Cursor::new(b"X: y\r\n\r\n{}".to_vec());
        assert!(read_message(&mut r).is_err());
    }
}
