use flate2::write::ZlibDecoder;
use std::io;
use std::io::prelude::*;
use std::io::{Cursor, Write};
pub(crate) fn decompress(bytes: Vec<u8>) -> Vec<u8> {
    let mut writer = Vec::new();
    let mut z = ZlibDecoder::new(writer);
    z.write_all(&bytes[..]);
    writer = z.finish().unwrap();
    writer.to_vec()
}

fn decode_reader(bytes: Vec<u8>) -> io::Result<String> {
    let mut writer = Vec::new();
    let mut z = ZlibDecoder::new(writer);
    z.write_all(&bytes[..])?;
    writer = z.finish()?;
    let return_string = String::from_utf8(writer).expect("String parsing error");
    Ok(return_string)
}
