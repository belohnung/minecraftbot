use crate::game::ConnectionState;
use crate::packets::{player, pos};
use crate::protocol::Packet;
use mc_varint::{VarIntRead, VarIntWrite};
use serde_json::json;
use std::collections::LinkedList;
use std::io::Result;
use std::io::{Cursor, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod game;
mod packets;
mod protocol;

fn main() {
    for i in 0..50 {
        thread::spawn(move || {
            bot(format!("soos{}", i).parse().unwrap());
        });
    }
    loop {}
}

fn bot(name: String) {
    //  let mut loggedin = false;
    let ipadress = "5.181.151.65";
    let port = 25565;
    let mut state = ConnectionState::Login;
    match TcpStream::connect(format!("{}:{}", ipadress, port)) {
        Ok(mut stream) => {
            let stream = Arc::new(Mutex::new(stream));
            let mut x = 0.0;
            {
                let stream2 = stream.clone();
                thread::spawn(move || loop {
                    Packet::ClientPlayerPositionAndLook {
                        x: 0.545,
                        y: 1.000000,
                        z: 0.0,
                        yaw: x,
                        pitch: 0.0,
                        onground: false,
                    }
                    .serialize(&mut *stream2.lock().unwrap())
                    .unwrap();
                    println!("tick  {}", x);
                    x = x + 0.001;
                });
            }

            println!("Successfully connected to server {}:{}", ipadress, port);
            // handshake, join
            Packet::ClientHandshake {
                host_address: ipadress.to_string(),
                port,
            }
            .serialize(&mut *stream.lock().unwrap());
            Packet::ClientJoin {
                player_name: name.to_string(),
            }
            .serialize(&mut *stream.lock().unwrap());

            'outer: loop {
                let received_packet = match Packet::deserialize(&mut *stream.lock().unwrap(), state)
                {
                    Ok(p) => p,
                    Err(e) => {
                        //eprintln!("error: {}", e);
                        continue 'outer;
                    }
                };

                match received_packet {
                    Packet::ServerLoginSuccess { name, uuid } => {
                        println!("Eingeloggt als {} mit der UUID: {:?}", name, uuid);
                        state = ConnectionState::Play;
                    }
                    Packet::ServerKeepAlive { magic: moom } => {
                        Packet::ClientKeepAlive { magic: moom }
                            .serialize(&mut *stream.lock().unwrap())
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
