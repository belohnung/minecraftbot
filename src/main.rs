use crate::game::{ConnectionState, Entity};
use crate::packets::{chat, player, pos};
use crate::protocol::{Packet, PacketError};
use mc_varint::{VarIntRead, VarIntWrite};
use serde_json::json;
use std::collections::LinkedList;
use std::io::{Cursor, Read, Write};
use std::io::{ErrorKind, Result};
use std::net::TcpStream;
use std::sync::mpsc::TryRecvError;
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
            let (inbound_sender, inbound_receiver) = mpsc::channel::<Packet>();

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
                        outbound_sender.send(buf);
                        //   println!("tick !");
                    }
                });
                thread::spawn({
                    || loop {
                        let mut vic = Vec::new();
                        match stream.read_exact(&mut vic) {
                            Ok(_) => {
                                let received_packet =
                                    match Packet::deserialize(&mut vic.as_slice(), state) {
                                        Ok(p) => p,
                                        Err(_) => (unimplemented!()),
                                    };
                                inbound_sender.send(received_packet);
                            }
                            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
                            Err(_) => {
                                println!("connection with server was severed");
                                break;
                            }
                        }
                        match outbound_receiver.try_recv() {
                            Ok(msg) => {
                                stream.write(&mut *msg);
                            }
                            Err(TryRecvError::Empty) => (),
                            Err(TryRecvError::Disconnected) => break,
                        }
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
                match inbound_receiver.try_recv().unwrap() {
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

            loop {}
        }
        _ => {}
    }

    println!("Terminated.");
}
