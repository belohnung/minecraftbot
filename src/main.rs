use crate::game::{ConnectionState, Entity};
use crate::packets::{chat, player, pos};
use crate::protocol::Packet;
use mc_varint::{VarIntRead, VarIntWrite};
use serde_json::json;
use std::collections::LinkedList;
use std::io::Result;
use std::io::{Cursor, Read, Write};
use std::net::TcpStream;
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

mod game;
mod packets;
mod protocol;

fn main() {
    bot("Fortz".parse().unwrap());
}

fn bot(name: String) {
    //  let mut loggedin = false;
    let ipadress = "45.154.51.160";
    let port = 25565;
    let mut state = ConnectionState::Login;
    let mut entity = Entity {
        entityid: 0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
        yaw: 0.0,
        pitch: 0.0,
    };
    match TcpStream::connect(format!("{}:{}", ipadress, port)) {
        Ok(mut stream) => {
            let (outbound_sender, outbound_receiver) = mpsc::channel::<Vec<u8>>();
            let (inbound_sender, inbound_receiver) = mpsc::channel::<Vec<u8>>();

            let entity = Arc::new(Mutex::new(entity));
            {
                thread::spawn({
                    let entity = entity.clone();

                    move || loop {
                        thread::sleep(Duration::from_millis(50));
                        let mut buf = Vec::new();
                        {
                            let lockedentity = entity.lock().unwrap();
                            Packet::ClientPlayerPositionAndLook {
                                x: lockedentity.x,
                                y: lockedentity.y,
                                z: lockedentity.z,
                                yaw: lockedentity.yaw,
                                pitch: lockedentity.pitch,
                                onground: false,
                            }
                            .serialize(&mut buf)
                            .unwrap();
                        }
                        sender.send(buf);
                        //   println!("tick !");
                    }
                });
            }

            println!("Successfully connected to server {}:{}", ipadress, port);
            // handshake, join
            Packet::ClientHandshake {
                host_address: ipadress.to_string(),
                port,
            }
            .serialize(&mut stream);
            Packet::ClientJoin {
                player_name: name.to_string(),
            }
            .serialize(&mut stream);

            'outer: loop {
                for packet in receiver.try_iter() {
                    // println!("sent packet");
                    stream.write_all(&packet);
                }
                let received_packet = match Packet::deserialize(&mut stream, state) {
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
                            .serialize(&mut stream)
                            .unwrap();

                        //stream.write(&chat("Koop Eliv"));
                    }
                    Packet::ServerPlayerPositionAndLook {
                        x,
                        y,
                        z,
                        yaw,
                        pitch,
                        flags,
                    } => {
                        let mut lockedentity = entity.lock().unwrap();
                        lockedentity.x = x;
                        lockedentity.y = y;
                        lockedentity.z = z;
                        lockedentity.yaw = yaw;
                        lockedentity.pitch = pitch;
                    }

                    p => println!("packet lol xd: {:#?}", p),
                }
            }
        }
        _ => {}
    }

    println!("Terminated.");
}
