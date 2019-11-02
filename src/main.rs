use crate::game::{CompressionStatus, ConnectionState, Entity, MinecraftConnection};
use crate::packets::{chat, player, pos};
use crate::protocol::PacketError::{DeserializeIOError, UnknownPacketIdentifier};
use crate::protocol::{Packet, PacketError};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TryRecvError};
use mc_varint::{VarIntRead, VarIntWrite};
use serde_json::json;
use std::collections::LinkedList;
use std::io::{Cursor, Read, Write};
use std::io::{ErrorKind, Result};
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Barrier, Mutex, RwLock};
use std::thread;
use std::time::Duration;

mod compression;
mod game;
mod packets;
mod protocol;

#[macro_use]
extern crate approx;

fn main() {
    bot("indian_techsupport".parse().unwrap());
}

fn bot(name: String) {
    //  let mut loggedin = false;
    let ipadress = "5.181.151.65";
    let port = 25565;
    let mut connection = Arc::new(RwLock::new(MinecraftConnection::new(
        format!("{}:{}", ipadress, port).parse().unwrap(),
        "gay".parse().unwrap(),
    )));
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
                    let connection = connection.clone();
                    let barrier = barrier.clone();
                    move || loop {
                        //
                        thread::sleep(Duration::from_millis(3));
                        let mut connection_state = { connection.write().unwrap() };
                        match outbound_receiver.recv_timeout(Duration::from_millis(1)) {
                            Ok(mut msg) => {
                                println!("-> {:X?}", msg);
                                match msg {
                                    p => {
                                        stream.write_all(&p.serialize(&connection_state).unwrap());
                                    }
                                }
                            }
                            Err(_) => {}
                            _ => {}
                        }
                        thread::sleep(Duration::from_millis(2));
                        match Packet::deserialize(&mut stream, &connection_state) {
                            Ok(received_packet) => match received_packet {
                                Packet::ServerCompressionLevelSet { compression_level } => {
                                    connection_state.compression =
                                        CompressionStatus::Enabled(compression_level);
                                    println!("Compression threshold set to {}", compression_level);
                                }
                                p => {
                                    inbound_sender
                                        .send_timeout(p.clone(), Duration::from_millis(1));
                                    //   println!(" <- {:02X?}", p);
                                }
                            },
                            Err(PacketError::SockySockyNoBlocky) => (),
                            Err(DeserializeIOError(err)) => {
                                //dbg!(err);
                            }
                            Err(PacketError::UnknownPacketIdentifier { id }) => {
                                //dbg!(err);
                            }
                            Err(err) => {
                                dbg!(err);
                            }
                            _ => (),
                        }

                        //  barrier.wait();
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
                let connection = connection.clone();
                let mut serverentity = Entity {
                    entityid: 0,
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    yaw: 0.0,
                    pitch: 0.0,
                };

                move || loop {
                    {
                        thread::sleep(Duration::from_millis(20));
                        //   barrier.wait();
                        let state = connection.read().unwrap().state;
                        let lockedentity = entity.lock().unwrap();

                        match state {
                            ConnectionState::Play => {
                                if !compareLoc(&lockedentity, &serverentity) {
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

                                ()
                            }

                            _ => {}
                        }
                    }

                    //   println!("tick !");
                }
            });
            'outer: loop {
                // barrier.wait();
                if let Ok(packet) = inbound_receiver.recv() {
                    let connection_state = { connection.read().unwrap().state };
                    match connection_state {
                        ConnectionState::Play => {
                            match packet {
                                Packet::ServerKeepAlive { magic: moom } => {
                                    outbound_sender.send(Packet::ClientKeepAlive { magic: moom });

                                    // entity.lock().unwrap().z += 1.0;
                                    // entity.lock().unwrap().pitch += 1.0;
                                    //  outbound_sender.send(chat("Koop Eliv"));
                                }

                                Packet::ServerChatPacket {
                                    message: msg,
                                    position: displayposition,
                                } => {
                                    entity.lock().unwrap().z += 1.0;
                                    dbg!(msg);
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
                                p => (),
                            }
                        }
                        ConnectionState::Login => match packet {
                            Packet::ServerLoginSuccess { name, uuid } => {
                                connection.write().unwrap().state = ConnectionState::Play;
                                println!("Logged in as {} with UUID: {:?}", name, uuid);
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
