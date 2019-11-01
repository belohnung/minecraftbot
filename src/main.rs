use crate::game::{ConnectionState, Entity};
use crate::packets::{chat, player, pos};
use crate::protocol::{Packet, PacketError};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use mc_varint::{VarIntRead, VarIntWrite};
use serde_json::json;
use std::collections::LinkedList;
use std::io::{Cursor, Read, Write};
use std::io::{ErrorKind, Result};
use std::net::TcpStream;
use std::sync::{Arc, Barrier, Mutex, RwLock};
use std::thread;
use std::time::Duration;

mod game;
mod packets;
mod protocol;

#[macro_use]
extern crate approx;

fn main() {
    bot("wtf".parse().unwrap());
}

fn bot(name: String) {
    //  let mut loggedin = false;
    let ipadress = "5.181.151.65";
    let port = 25565;
    let mut state = Arc::new(RwLock::new(ConnectionState::Login));
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
            stream.set_nonblocking(true);
            let barrier = Arc::new(Barrier::new(2));
            let (outbound_sender, outbound_receiver) = crossbeam_channel::unbounded::<Packet>();
            let (inbound_sender, inbound_receiver) = crossbeam_channel::unbounded::<Packet>();

            let entity = Arc::new(Mutex::new(entity));
            {
                thread::spawn({
                    let state = state.clone();
                    let barrier = barrier.clone();
                    move || loop {
                        let connection_state = { *state.read().unwrap() };
                        //  barrier.wait();
                        match outbound_receiver.try_recv() {
                            Ok(mut msg) => {
                                println!("-> {:X?}", msg);
                                stream.write_all(&msg.serialize().unwrap());
                            }
                            Err(TryRecvError::Empty) => (),
                            Err(TryRecvError::Disconnected) => break,
                        }

                        //  dbg!(&*state.read().unwrap());
                        thread::sleep(Duration::from_millis(50));
                        //  println!("NIGGI_______________________________________");
                        match Packet::deserialize(&mut stream, &connection_state) {
                            Ok(received_packet) => {
                                println!(" <- {:X?}", received_packet);

                                inbound_sender.send(received_packet);
                            }
                            //Err(PacketError::SockySockyNoBlocky) => (),
                            Err(err) => (),
                            _ => (),
                        }

                        //   println!("NIGGI______________OLILI_________________________")
                    }
                });
            }

            println!("Successfully connected to server {}:{}", ipadress, port);
            // handshake, join
            outbound_sender.send(Packet::ClientHandshake {
                host_address: ipadress.to_string(),
                port,
            });
            outbound_sender.send(Packet::ClientJoin {
                player_name: name.to_string(),
            });

            thread::spawn({
                let entity = entity.clone();
                let outbound_sender = outbound_sender.clone();
                let state = state.clone();

                move || loop {
                    {
                        thread::sleep(Duration::from_millis(20));
                        //   barrier.wait();
                        let lockedentity = entity.lock().unwrap();
                        let mut serverentity = Entity {
                            entityid: 0,
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                            yaw: 0.0,
                            pitch: 0.0,
                        };

                        match *state.read().unwrap() {
                            ConnectionState::Play => {
                                /*  if !compareLoc(&lockedentity, &serverentity) {
                                    outbound_sender.send(Packet::ClientPlayerPositionAndLook {
                                        x: lockedentity.x,
                                        y: lockedentity.y,
                                        z: lockedentity.z,
                                        yaw: lockedentity.yaw,
                                        pitch: lockedentity.pitch,
                                        onground: false,
                                    });
                                    serverentity = **&lockedentity;
                                }
                                */
                                ()
                            }

                            _ => {}
                        }
                    }

                    //   println!("tick !");
                }
            });
            'outer: loop {
                if let Ok(packet) = inbound_receiver.try_recv() {
                    let connection_state = { *state.read().unwrap() };
                    match connection_state {
                        ConnectionState::Play => {
                            match packet {
                                Packet::ServerKeepAlive { magic: moom } => {
                                    println!("Keeeeep {}", moom);

                                    outbound_sender.send(Packet::ClientKeepAlive { magic: moom });

                                    // entity.lock().unwrap().z += 1.0;
                                    // entity.lock().unwrap().pitch += 1.0;
                                    //  outbound_sender.send(chat("Koop Eliv"));
                                }

                                Packet::ServerPlayerPositionAndLook {
                                    x,
                                    y,
                                    z,
                                    yaw,
                                    pitch,
                                    flags,
                                } => {
                                    println!("packet position bekommen");
                                    let mut lockedentity = entity.lock().unwrap();
                                    lockedentity.x = x;
                                    lockedentity.y = y;
                                    lockedentity.z = z;
                                    lockedentity.yaw = yaw;
                                    lockedentity.pitch = pitch;
                                    println!("eigene position angepasst bro");
                                }
                                p => println!("packet lol xd: {:#?}", p),
                            }
                        }
                        ConnectionState::Login => match packet {
                            Packet::ServerLoginSuccess { name, uuid } => {
                                println!("Eingeloggt als {} mit der UUID: {:?}", name, uuid);

                                *state.write().expect("gay") = ConnectionState::Play;
                                println!("uwwu");

                                println!("uwu");
                            }
                            p => {
                                dbg!(p);
                            }
                        },
                        _ => (),
                    }
                }
            }
        }
        _ => {}
    }

    println!("Terminated.");
}

fn compareLoc(entity1: &Entity, entity2: &Entity) -> bool {
    let mut out = false;
    out = eq(entity1.x, entity2.x, 1.0);
    out = eq(entity1.y, entity2.y, 1.0);
    out = eq(entity1.z, entity2.z, 1.0);
    out
}

fn eq(a: f64, b: f64, range: f64) -> bool {
    (a - b).abs() <= range
}
