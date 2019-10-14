use mc_varint::{VarIntRead, VarIntWrite};

pub fn chat(message: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.write_var_i32(message.len() as i32 + 2).unwrap(); // packetlength
    buf.write_var_i32(0x01).unwrap(); // type ??
    buf.write_var_i32(message.len() as i32).unwrap(); // type ??
    buf.extend_from_slice(message.as_bytes()); // stringlength
    buf
}
pub fn animation() -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.write_var_i32(1).unwrap(); // packetlength
    buf.write_var_i32(0x0A).unwrap(); // type ??
    buf
}
pub fn pos_and_looking(x: f64, y: f64, z: f64, yaw: f32, pitch: f32, onground: bool) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let mut len: Vec<u8> = Vec::new();

    // packetlength
    buf.write_var_i32(0x06).unwrap(); // type ??
    buf.extend_from_slice(&x.to_bits().to_be_bytes());
    buf.extend_from_slice(&y.to_bits().to_be_bytes());
    buf.extend_from_slice(&z.to_bits().to_be_bytes());
    buf.extend_from_slice(&yaw.to_bits().to_be_bytes());
    buf.extend_from_slice(&pitch.to_bits().to_be_bytes());
    buf.push(onground as u8);
    len.write_var_i32(buf.len() as i32).unwrap();
    len.extend_from_slice(&buf);
    len
}
pub fn pos(x: f64, y: f64, z: f64, onground: bool) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let mut len: Vec<u8> = Vec::new();

    // packetlength
    buf.write_var_i32(0x04).unwrap(); // type ??
    buf.extend_from_slice(&x.to_bits().to_be_bytes());
    buf.extend_from_slice(&y.to_bits().to_be_bytes());
    buf.extend_from_slice(&z.to_bits().to_be_bytes());
    buf.push(onground as u8);
    len.write_var_i32(buf.len() as i32).unwrap();
    len.extend_from_slice(&buf);
    len
}
pub fn player(onground: bool) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let mut len: Vec<u8> = Vec::new();

    // packetlength
    buf.write_var_i32(0x03).unwrap(); // type ??
    buf.push(onground as u8);
    len.write_var_i32(buf.len() as i32).unwrap();
    len.extend_from_slice(&buf);
    len
}
