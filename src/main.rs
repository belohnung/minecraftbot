use crate::game::{ConnectionState, Entity};
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
    bot("Nogga".parse().unwrap());
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
            let stream = Arc::new(Mutex::new(stream));
            let entity = Arc::new(Mutex::new(entity));
            {
                let stream2 = stream.clone();
                let entity2 = entity.clone();
                thread::spawn(move || loop {
                    thread::sleep(Duration::from_millis(50));
                    let lockedentity = entity2.lock().unwrap();
                    Packet::ClientPlayerPositionAndLook {
                        x: lockedentity.x,
                        y: lockedentity.y,
                        z: lockedentity.z,
                        yaw: lockedentity.yaw,
                        pitch: lockedentity.pitch,
                        onground: false,
                    }
                    .serialize(&mut *stream2.lock().unwrap())
                    .unwrap();
                    println!("tick !");
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
                        let mut locked = entity.lock().unwrap();
                        locked.z += 1.0;
                        locked.x += 1.0;
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
