#[macro_use]
extern crate approx;
use crate::game::{CompressionStatus, ConnectionState, Entity, MinecraftConnection};
use crate::packets::{player, pos};
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
use std::thread::sleep;
use std::time::Duration;

#[macro_use]
mod macros;
mod compression;
mod game;
mod hash;
mod packets;
mod protocol;
//mod world;

fn main() {
    bot("owow".parse().unwrap());
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

    match TcpStream::connect(format!("{}:{}", "dev.blohnung.de", "25565")) {
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
                        // barrier.wait();

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
                        thread::sleep(Duration::from_millis(1));
                        match Packet::deserialize(&mut stream, &connection_state) {
                            Ok(received_packet) => match received_packet.clone() {
                                Packet::ServerCompressionLevelSet { compression_level } => {
                                    if let CompressionStatus::Enabled(i) =
                                        connection_state.compression
                                    {
                                        println!("Compression threshold wants to be set to {} but already was set to {}", compression_level, i);
                                    } else {
                                        connection_state.compression =
                                            CompressionStatus::Enabled(compression_level);
                                        println!(
                                            "Compression threshold set to {}",
                                            compression_level
                                        );
                                    }
                                }
                                Packet::ServerLoginSuccess { uuid, name } => {
                                    inbound_sender.send(received_packet);
                                    // thread::sleep(Duration::from_millis(1));
                                }
                                p => {
                                    inbound_sender.send(p.clone());
                                    println!(" <- {:02X?}", p);
                                }
                            },
                            Err(DeserializeIOError(err)) => {
                                //dbg!(err);
                            }
                            Err(PacketError::UnknownPacketIdentifier { .. }) => {
                                //dbg!(err);
                            }
                            Err(err) => {
                                dbg!(err);
                            }
                            _ => (),
                        }

                        //  println!("etron");
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
                let mut enabled = false;
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
                        //  dbg!("kot");
                        thread::sleep(Duration::from_millis(20));
                        //   barrier.wait();
                        //   barrier.wait();
                        // println!("as");
                        {
                            if !enabled {
                                let state = connection.read().unwrap().state;

                                match state {
                                    ConnectionState::Play => {
                                        enabled = true;
                                    }

                                    _ => {}
                                }
                            } else {
                                let lockedentity = entity.lock().unwrap();
                                if !compareLoc(&lockedentity, &serverentity) {
                                    outbound_sender
                                        .send(Packet::ClientPlayerPositionAndLook {
                                            x: lockedentity.x,
                                            y: lockedentity.y,
                                            z: lockedentity.z,
                                            yaw: lockedentity.yaw,
                                            pitch: lockedentity.pitch,
                                            onground: false,
                                        })
                                        .unwrap();
                                    serverentity = **&lockedentity;
                                }
                            }
                        }
                    }

                    //   println!("tick !");
                }
            });
            'outer: loop {
                //thread::sleep(Duration::from_millis(5));
                //
                // barrier.wait();
                if let Ok(packet) = inbound_receiver.try_recv() {
                    println!(".");
                    match packet {
                        Packet::ServerKeepAlive { magic: moom } => {
                            outbound_sender.send(Packet::ClientKeepAlive { magic: moom });

                            // entity.lock().unwrap().z += 1.0;
                            // entity.lock().unwrap().pitch += 1.0;
                            //outbound_sender.send(chat("Koop Eliv"));
                        }

                        Packet::ServerChatPacket {
                            message: msg,
                            position: displayposition,
                        } => {
                            let mut gay = msg.as_str();
                            if !gay.contains("!") {
                                let message = msg.split("\\u003e").nth(1).unwrap_or("> gay");
                                dbg!(message);
                                let mut bot = entity.lock().unwrap();
                                if message.contains("w") {
                                    bot.z += 1.0;
                                }
                                if message.contains("a") {
                                    bot.x += 1.0;
                                }
                                if message.contains("d") {
                                    bot.x -= 1.0;
                                }
                                if message.contains("s") {
                                    bot.z -= 1.0;
                                }
                                bot.yaw += 10.0;
                                //  bot.pitch = -90.0;
                                dbg!(&msg);
                            }
                            if gay.contains("#") {
                                let mut bot = entity.lock().unwrap();
                                let message: Vec<&str> = gay
                                    .split("\\u003e")
                                    .nth(1)
                                    .unwrap_or("\"]")
                                    .split("\"")
                                    .nth(0)
                                    .unwrap()
                                    .split("\"")
                                    .collect();
                                let command = message[0];
                                dbg!(message);
                                if command.contains("setslot") {
                                    outbound_sender.send(Packet::ClientHeldItemChange { slot: 5 });
                                }
                            }
                        }

                        Packet::ServerPlayerPositionAndLook {
                            x,
                            y,
                            z,
                            yaw,
                            pitch,
                            flags,
                            teleportid,
                        } => {
                            /* outbound_sender.send(Packet::ClientChat {
                                message: "!da is ne wand".to_string(),
                            });*/
                            println!("packet position bekommen");
                            let mut lockedentity = entity.lock().unwrap();
                            lockedentity.x = x;
                            lockedentity.y = y;
                            lockedentity.z = z;
                            lockedentity.yaw = yaw;
                            lockedentity.pitch = pitch;
                            println!("eigene position angepasst bro");
                        }

                        Packet::ServerLoginSuccess { name, uuid } => {
                            connection.write().unwrap().state = ConnectionState::Play;
                            println!("Logged in as {} with UUID: {:?}", name, uuid);
                        }
                        p => {
                            dbg!(p);
                        }
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
    out = eq(entity1.x, entity2.x, 0.5);
    if (out) {
        out = eq(entity1.y, entity2.y, 0.5);
    }
    if (out) {
        out = eq(entity1.z, entity2.z, 0.5);
    }

    //kann auch mit && gemacht werden bro

    out
}

fn eq(a: f64, b: f64, range: f64) -> bool {
    (a - b).abs() <= range
}
