use sha1::{Digest, Sha1};

use std::iter;

use regex::Regex;
use rustc_serialize::hex::ToHex;
use sha1::digest::DynDigest;

const LEADING_ZERO_REGEX: &str = r#"^0+"#;

fn calc_hash(name: &str) -> String {
    let mut hasher = Sha1::new();
    sha1::Digest::input(&mut hasher, name.as_bytes());
    //  let mut hex: Vec<u8> = iter::repeat(0)
    //      .take((hasher.output_size() + 7) / 8)
    //      .collect();
    let mut hex = hasher.result();

    let negative = (hex[0] & 0x80) == 0x80;

    let regex = Regex::new(LEADING_ZERO_REGEX).unwrap();

    if negative {
        two_complement(&mut hex.to_vec());
        format!(
            "-{}",
            regex
                .replace(hex.as_slice().to_hex().as_str(), "")
                .to_string()
        )
    } else {
        regex
            .replace(hex.as_slice().to_hex().as_str(), "")
            .to_string()
    }
}

fn two_complement(bytes: &mut Vec<u8>) {
    let mut carry = true;
    for i in (0..bytes.len()).rev() {
        bytes[i] = !bytes[i] & 0xff;
        if carry {
            carry = bytes[i] == 0xff;
            bytes[i] = bytes[i] + 1;
        }
    }
}
