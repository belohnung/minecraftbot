mod structs;
use crate::structs::Entity;
use mc_varint::{VarIntRead, VarIntWrite};
use serde_json::json;
use std::io::Result;
use std::io::{Cursor, Read, Write};
use std::net::TcpStream;
use structs::Location;

fn main() {
    let mut loggedin = false;
    let ipadress = "5.181.151.65:25565";
    let mut me = Entity {
        id: 0,
        position: Location { x: 0, y: 0, z: 0 },
    };
    match TcpStream::connect(ipadress) {
        Ok(mut stream) => {
            println!("Successfully connected to server {:?}", ipadress);
            stream.write(handshake(ipadress).as_slice());
            stream.write(join("Rustlang").as_slice());

            'outer: loop {
                let length = read_varint(&mut stream).unwrap().0;
                let mut rest = vec![0u8; length as usize];
                stream.read_exact(&mut rest);
                // println!("decoded varints: {:?}", read_varints(&rest));
                let mut rest = Cursor::new(&rest);

                //  println!("received packet with length {}", length);

                if !loggedin {
                    let packet_type = read_varint(&mut rest).unwrap().0;
                    let datalength = read_varint(&mut rest).unwrap_or((0, 0)).0 as usize;
                    /*
                    println!(
                        "length: {}, packet id: {}, datalength: {}",
                        length, packet_type, datalength
                    );
                    */
                    /* println!(
                        "PacketID: {} Length: {:?} Datalen:{:?} ",
                        packet, length, datalength
                    );*/
                    match packet_type {
                        2 => {
                            let uuid = read_string(&mut rest, datalength);
                            let datalength = read_varint(&mut rest).unwrap().0 as usize;
                            let name = read_string(&mut rest, datalength);
                            println!("Logged in with Name: {} and UUID: {:?}", name, uuid);
                            loggedin = true;
                        }
                        3 => {
                            println!("Compression Level set to {:?}", datalength);
                            println!("Compression ist aber nicht supportet momentan :((");
                            //break 'outer;
                        }
                        _ => (),
                    }
                } else {
                    let packet_type = read_varint(&mut rest).unwrap().0;
                    /*
                    println!(
                        "PacketID: {:?} Length: {:?} Datalen:{:?} Data:{:X?}",
                        packet_type,
                        length,
                        datalength,
                        read_to_buff(&mut rest, datalength)
                    );
                    */
                    match packet_type {
                        0x01 => {
                            let mut bit = [0; 4];
                            rest.read_exact(&mut bit)
                                .expect("Could not read Position Packet");
                            let entityid = u32::from_be_bytes(bit);
                            me.id = entityid;
                            let mut bit = [0; 1];
                            rest.read_exact(&mut bit)
                                .expect("Could not read Position Packet");
                            let gamemode = u8::from_be_bytes(bit);
                            let mut bit = [0; 1];
                            rest.read_exact(&mut bit)
                                .expect("Could not read Position Packet");
                            let dimension = i8::from_be_bytes(bit);
                            let mut bit = [0; 1];
                            rest.read_exact(&mut bit)
                                .expect("Could not read Position Packet");
                            let difficulty = u8::from_be_bytes(bit);
                            let mut bit = [0; 1];
                            rest.read_exact(&mut bit)
                                .expect("Could not read Position Packet");
                            let maxplayers = u8::from_be_bytes(bit);
                            println!("Joined the Game! Gameinfo: Gamemode:{:?}, Dimension:{:?}, Difficulty:{:?}, Maxplayers:{:?}",gamemode, dimension, difficulty, maxplayers)
                        }
                        0x00 => {
                            let magic = read_varint(&mut rest).expect("malformed packet");

                            stream.write_all(&chat("KeepAlive"));
                            stream.write_all(&keepalive(magic.0, magic.1));
                        }
                        0x05 => {
                            let mut bit = [0; 8];
                            rest.read_exact(&mut bit)
                                .expect("Could not read Position Packet");
                            let mut val = u64::from_be_bytes(bit);
                            let pos = Location::from_long(val);

                            println!(
                                "Spawnposition: X: {:?}, Y: {:?}, Z: {:?}",
                                pos.x, pos.y, pos.z
                            );
                        }
                        _ => (),
                    }
                }
            }
        }
        _ => {}
    }

    println!("Terminated.");
}

fn read_varints(input: &[u8]) -> Vec<i32> {
    let mut vec = Vec::new();
    let mut input = Cursor::new(input);
    while let Ok((varint, offset)) = read_varint(&mut input) {
        vec.push(varint);
    }
    vec
}

fn read_varint<R>(stream: &mut R) -> Result<(i32, usize)>
where
    R: Read,
{
    let mut varintbuf = vec![];
    loop {
        let mut byte = [0];
        stream.read_exact(&mut byte)?;

        varintbuf.push(byte[0]);
        if byte[0] & 0x80 == 0 {
            break;
        }
    }
    let len = varintbuf.len();
    Ok((Cursor::new(varintbuf).read_var_i32()?, len))
}

fn read_string<R>(stream: &mut R, len: usize) -> String
where
    R: Read,
{
    let mut varintbuf = vec![0; len];

    stream.read_exact(varintbuf.as_mut_slice()).unwrap();

    String::from_utf8_lossy(&varintbuf).to_string()
}

fn join(name: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.write_var_i32((name.len() + 2) as i32).unwrap(); // packetlength
    buf.write_var_i32(0).unwrap(); // type ??
    buf.write_var_i32(name.len() as i32).unwrap(); // stringlength
    buf.extend_from_slice(name.as_bytes());
    buf
}

fn handshake(host: &str) -> Vec<u8> {
    let host_port = host.split(":").collect::<Vec<&str>>();
    let port = match host_port.get(1) {
        Some(port_string) => port_string.parse::<u16>().unwrap_or(25565),
        None => 25565,
    };

    let host = host_port[0];

    let mut buf: Vec<u8> = Vec::new();
    buf.write_var_i32((host.len() + 2 + 2 + 2) as i32).unwrap(); // packetlength
    buf.write_var_i32(0).unwrap(); // type ??
    buf.write_var_i32(47).unwrap(); //protocol version
    buf.write_var_i32(host.len() as i32).unwrap(); // stringlength
    buf.extend_from_slice(host.as_bytes()); // ip
    buf.extend(port.to_be_bytes().iter()); // port
    buf.write_var_i32(2).unwrap(); // next stage
    buf
}

fn keepalive(magic: i32, length: usize) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.write_var_i32((length) as i32).unwrap(); // packetlength
    buf.write_var_i32(0).unwrap(); // type ??
    buf.write_var_i32(magic).unwrap(); // stringlength
    buf
}

fn chat(message: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.write_var_i32(message.len() as i32 + 2).unwrap(); // packetlength
    buf.write_var_i32(0x01).unwrap(); // type ??
    buf.write_var_i32(message.len() as i32).unwrap(); // type ??
    buf.extend_from_slice(message.as_bytes()); // stringlength
    buf
}

fn read_to_buff<R>(stream: &mut R, len: usize) -> Vec<u8>
where
    R: Read,
{
    let mut varintbuf = vec![0; len];
    stream.read_exact(&mut varintbuf).unwrap();
    varintbuf
}
