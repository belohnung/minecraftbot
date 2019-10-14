use crate::game::ConnectionState;
use crate::protocol::Packet;
use mc_varint::{VarIntRead, VarIntWrite};
use serde_json::json;
use std::io::Result;
use std::io::{Cursor, Read, Write};
use std::net::TcpStream;

mod game;
mod packets;
mod protocol;

fn main() {
    //  let mut loggedin = false;
    let ipadress = "5.181.151.65";
    let port = 25565;
    let mut state = ConnectionState::Login;
    match TcpStream::connect(format!("{}:{}", ipadress, port)) {
        Ok(mut stream) => {
            println!("Successfully connected to server {}:{}", ipadress, port);
            // handshake, join
            Packet::ClientHandshake {
                host_address: ipadress.to_string(),
                port,
            }
            .serialize(&mut stream);
            Packet::ClientJoin {
                player_name: "N".to_string(),
            }
            .serialize(&mut stream);

            'outer: loop {
                match match Packet::deserialize(&mut stream, state) {
                    Ok(p) => p,
                    Err(e) => {
                        //eprintln!("error: {}", e);
                        continue 'outer;
                    }
                } {
                    Packet::ServerLoginSuccess { name, uuid } => {
                        println!("Eingeloggt als {} mit der UUID: {:?}", name, uuid);
                        state = ConnectionState::Play;
                    }
                    Packet::ServerKeepAlive { magic: moom } => {
                        Packet::ClientKeepAlive { magic: moom }
                            .serialize(&mut stream)
                            .unwrap();
                    }

                    p => println!("packet lol xd: {:#?}", p),
                }
            }
        }
        _ => {}
    }

    println!("Terminated.");
}
