use crate::compression::*;
use crate::game::CompressionStatus::Enabled;
use crate::game::{CompressionStatus, ConnectionState, MinecraftConnection};
#[macro_use]
use crate::macros;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use err_derive::Error;
use flate2::read::{ZlibDecoder, ZlibEncoder};
use flate2::Compression;
use mc_varint::{VarIntRead, VarIntWrite};
use std::borrow::BorrowMut;
use std::io;
use std::io::{Cursor, Error, Read, Result as IOResult, Write};
use std::result::Result;

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub enum RawPacketValueType {
    byte,
    ubyte,
    short,
    ushort,
    int,
    long,
    varint,
    varlong,
    float,
    double,
    int128,
    String,
    Position,
    boolean,
}

impl RawPacketValueType {
    pub fn from_buf<R>(&self, buf: &mut R) -> IOResult<RawPacketValue>
    where
        R: Read,
    {
        Ok(match self {
            RawPacketValueType::byte => RawPacketValue::byte(buf.read_i8()?),
            RawPacketValueType::ubyte => RawPacketValue::ubyte(buf.read_u8()?),
            RawPacketValueType::short => RawPacketValue::short(buf.read_i16::<BigEndian>()?),
            RawPacketValueType::ushort => RawPacketValue::ushort(buf.read_u16::<BigEndian>()?),
            RawPacketValueType::int => RawPacketValue::int(buf.read_i32::<BigEndian>()?),
            RawPacketValueType::long => RawPacketValue::long(buf.read_i64::<BigEndian>()?),
            RawPacketValueType::varint => RawPacketValue::varint(buf.read_var_i32()?),
            RawPacketValueType::varlong => RawPacketValue::varlong(buf.read_var_i64()?),
            RawPacketValueType::float => RawPacketValue::float(buf.read_f32::<BigEndian>()?),
            RawPacketValueType::double => RawPacketValue::double(buf.read_f64::<BigEndian>()?),
            RawPacketValueType::int128 => RawPacketValue::int128(buf.read_i128::<BigEndian>()?),
            RawPacketValueType::String => {
                let len = buf.read_var_i32()?;
                let mut sbuf = vec![0u8; len as usize];
                buf.read_exact(&mut sbuf);
                RawPacketValue::String(String::from_utf8_lossy(&sbuf).to_string())
            }
            RawPacketValueType::Position => {
                let val = buf.read_i64::<BigEndian>()?;
                RawPacketValue::Position(val >> 38, val << 26 >> 52, val << 38 >> 38)
            }
            RawPacketValueType::boolean => RawPacketValue::boolean(buf.read_u8()? != 0),
        })
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum RawPacketValue {
    byte(i8),
    ubyte(u8),
    short(i16),
    ushort(u16),
    int(i32),
    long(i64),
    varint(i32),
    varlong(i64),
    float(f32),
    double(f64),
    int128(i128),
    String(String),
    Position(i64, i64, i64),
    boolean(bool),
}

impl RawPacketValue {
    pub fn serialize<W>(&self, buf: &mut W) -> IOResult<()>
    where
        W: Write,
    {
        Ok(match &self {
            RawPacketValue::byte(v) => buf.write_i8(*v)?,
            RawPacketValue::ubyte(v) => buf.write_u8(*v)?,
            RawPacketValue::short(v) => buf.write_i16::<BigEndian>(*v)?,
            RawPacketValue::ushort(v) => buf.write_u16::<BigEndian>(*v)?,
            RawPacketValue::int(v) => buf.write_i32::<BigEndian>(*v)?,
            RawPacketValue::varint(v) => buf.write_var_i32(*v).map(drop)?,
            RawPacketValue::varlong(v) => buf.write_var_i64(*v).map(drop)?,
            RawPacketValue::float(v) => buf.write_f32::<BigEndian>(*v)?,
            RawPacketValue::double(v) => buf.write_f64::<BigEndian>(*v)?,
            RawPacketValue::int128(v) => buf.write_i128::<BigEndian>(*v)?,
            RawPacketValue::String(v) => {
                buf.write_var_i32(v.len() as i32)?;
                buf.write_all(v.as_bytes())?;
            }
            RawPacketValue::Position(x, y, z) => buf.write_i64::<BigEndian>(
                (x & 0x3FFFFFF) << 38 | (y & 0xFFF) << 26 | (z & 0x3FFFFFF),
            )?,
            RawPacketValue::boolean(v) => buf.write_u8(if *v { 1 } else { 0 })?,
            RawPacketValue::long(v) => buf.write_i64::<BigEndian>(*v)?,
        })
    }
}

#[derive(Debug, Error)]
pub enum PacketError {
    #[error(
        display = "packet not implemented for state '{:?}': ID 0x{:02X}",
        state,
        id
    )]
    UnknownPacketIdentifier { id: i32, state: ConnectionState },
    #[error(display = "malformed packet: {}", 0)]
    MalformedPacket(&'static str),
    #[error(display = "i/o error while deserializing packet: {:?}", 0)]
    DeserializeIOError(std::io::Error),
}

impl_packets! {
    Packet, PacketType,
    ///////// C -> S (serverbound)
    // Login state
    None, Server, 0x00, ClientHandshake {
        host_address: String,
        port: u16,
    },
    Login, Server, 0x00, ClientJoin {
        player_name: String,
    },

    // Status state
    // ..

    // Play state
    Play, Server, 0x0F, ClientKeepAlive {
        magic: i64,
    },
    Play, Server, 0x04, ClientPlayerPosition {
        x: f64,
        y: f64,
        z: f64,
        onground: bool,
    },
    Play, Server, 0x05 ,ClientPlayerLook {
        yaw: f32,
        pitch: f32,
        onground: bool,
    },
    Play, Server, 0x03, ClientChat {
        message: String,
    },
    Play, Server, 0x23 ,ClientHeldItemChange {
        slot: i16,
    },
    Play, Server, 0x06 ,ClientPlayerPositionAndLook {
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        pitch: f32,
        onground: bool,
    },
    ///////// S -> C (clientbound)
    // Login state
    Login, Client, 0x03 ,ServerCompressionLevelSet {
        compression_level: i32,
    },
     Login, Client, 0x02 ,ServerLoginSuccess {
        uuid: String,
        name: String,
    },
     Login, Client, 0x01 ,ServerEncryptionRequest {
        serverid: String,
        pubkey: Vec<u8>,
        verifytoken: Vec<u8>,
    },
    // Status state
    // ..

    // Play state
    Play, Client, 0x20 ,ServerKeepAlive {
        magic: i64,
    },
    Play, Client, 0x4E ,ServerWorldTimeUpdate {
        age: i64,
        time: i64,
    },
    Play, Client, 0x1A ,ServerDisconnectPacket {
        reason: String,
    },
    Play, Client, 0x0E ,ServerChatPacket {
        message: String,
        position: i8,
    },
    Play, Client, 0x25 ,ServerJoinGame {
        entity_id: i32,
        gamemode: u8,
        dimension: i8,
        difficulty: u8,
        max_players: u8,
        level_type: String,
        reduced_debug_info: bool,
    },
    Play, Client, 0x35 ,ServerPlayerPositionAndLook {
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        pitch: f32,
        flags: i8,
        teleportid: i32,
    },
}

impl Packet {
    /// Takes a received packet from the server and decodes it into a Packet
    pub fn deserialize<R>(
        buf: &mut R,
        connection: &MinecraftConnection,
    ) -> Result<Packet, PacketError>
    where
        R: Read,
    {
        let packet_len = buf
            .read_var_i32()
            .map_err(|e| PacketError::DeserializeIOError(e))?;
        let mut packet = vec![0u8; packet_len as usize];
        buf.read_exact(&mut packet)
            .map_err(|e| PacketError::DeserializeIOError(e))?;
        let mut packet_cursor = Cursor::new(packet.clone());
        let mut packet_data_cursor = Cursor::new(vec![0u8; packet_len as usize]);

        if let Enabled(compression_level) = connection.compression {
            let uncompressed_size = packet_cursor
                .read_var_i32()
                .map_err(|e| PacketError::DeserializeIOError(e))?;

            if uncompressed_size != 0 {
                let mut new = Vec::with_capacity(uncompressed_size as usize);
                {
                    let mut reader = ZlibDecoder::new(packet_cursor);
                    match reader.read_to_end(&mut new) {
                        Err(e) => {
                            dbg!(e);
                        }
                        Ok(_) => (),
                    };
                }

                packet_data_cursor = io::Cursor::new(new);
            } else {
                packet_data_cursor = packet_cursor.clone();
            }
        } else {
            packet_data_cursor = packet_cursor.clone();
        }

        let type_id = packet_data_cursor
            .read_var_i32()
            .map_err(|e| PacketError::DeserializeIOError(e))?;
        //println!("len: {:?}  data: {:X?}", packet_len, packet_data.clone());
        /*
                println!(
                    "typeid: 0x{:02X} len: {:?}  CompressionStatus: {:?} PlayState: {:?} \n data: {:?}",
                    type_id, packet_len, &connection.compression, &connection.state, packet_data_cursor
                );
        */
        // println!("state: {:10?}, id: 0x{:02X}", connection.state, type_id);

        let packet_type =
            PacketType::from_state_and_id_and_direction(connection.state, type_id, BoundTo::Client);

        if let Some(packet_type) = packet_type {
            println!("PacketType: {:?} ", packet_type);
            match packet_type {
                PacketType::ServerEncryptionRequest => {
                    let mut sbuf = vec![0u8; 20 as usize];
                    packet_data_cursor.read_exact(&mut sbuf);
                    let serverid = String::from_utf8_lossy(&sbuf).to_string();
                    let packet_fields = read_values_from_template(
                        &mut packet_data_cursor,
                        &[RawPacketValueType::varint, RawPacketValueType::varint],
                    )
                    .map_err(|io_err| PacketError::DeserializeIOError(io_err))?;

                    if let [RawPacketValue::varint(pubkeylen), RawPacketValue::varint(vertokenlen)] =
                        packet_fields.as_slice()
                    {
                        let mut pubkey = vec![0u8; *pubkeylen as usize];
                        packet_data_cursor.read_exact(&mut pubkey);
                        let mut verifytoken = vec![0u8; *vertokenlen as usize];
                        packet_data_cursor.read_exact(&mut verifytoken);

                        Ok(Packet::ServerEncryptionRequest {
                            serverid,
                            pubkey,
                            verifytoken,
                        })
                    } else {
                        Err(PacketError::MalformedPacket("neger"))
                    }
                }
                PacketType::ServerDisconnectPacket => {
                    let packet_fields = read_values_from_template(
                        &mut packet_data_cursor,
                        &[RawPacketValueType::String],
                    )
                    .map_err(|io_err| PacketError::DeserializeIOError(io_err))?;

                    if let [RawPacketValue::String(reason)] = packet_fields.as_slice() {
                        Ok(Packet::ServerDisconnectPacket {
                            reason: reason.clone(),
                        })
                    } else {
                        Err(PacketError::MalformedPacket("join packet romped"))
                    }
                }
                // state:Login
                PacketType::ServerLoginSuccess => {
                    let packet_fields = read_values_from_template(
                        &mut packet_data_cursor,
                        &[RawPacketValueType::String, RawPacketValueType::String],
                    )
                    .map_err(|io_err| PacketError::DeserializeIOError(io_err))?;

                    if let [RawPacketValue::String(uuid), RawPacketValue::String(name)] =
                        packet_fields.as_slice()
                    {
                        Ok(Packet::ServerLoginSuccess {
                            uuid: uuid.clone(),
                            name: name.clone(),
                        })
                    } else {
                        Err(PacketError::MalformedPacket("join packet romped"))
                    }
                }

                PacketType::ServerCompressionLevelSet => {
                    let packet_fields = read_values_from_template(
                        &mut packet_data_cursor,
                        &[RawPacketValueType::varint],
                    )
                    .map_err(|io_err| PacketError::DeserializeIOError(io_err))?;

                    if let [RawPacketValue::varint(compression_level)] = packet_fields.as_slice() {
                        Ok(Packet::ServerCompressionLevelSet {
                            compression_level: *compression_level,
                        })
                    } else {
                        Err(PacketError::MalformedPacket("compression packet romped"))
                    }
                }

                //  state: Play
                PacketType::ServerKeepAlive => {
                    let packet_fields = read_values_from_template(
                        &mut packet_data_cursor,
                        &[RawPacketValueType::long],
                    )
                    .map_err(|io_err| PacketError::DeserializeIOError(io_err))?;

                    if let [RawPacketValue::long(magic)] = packet_fields.as_slice() {
                        Ok(Packet::ServerKeepAlive { magic: *magic })
                    } else {
                        Err(PacketError::MalformedPacket("KeepAlive packet was BRRRRed"))
                    }
                }
                PacketType::ServerJoinGame => {
                    let packet_fields = read_values_from_template(
                        &mut packet_data_cursor,
                        &[
                            RawPacketValueType::int,
                            RawPacketValueType::ubyte,
                            RawPacketValueType::byte,
                            RawPacketValueType::ubyte,
                            RawPacketValueType::ubyte,
                            RawPacketValueType::String,
                            RawPacketValueType::boolean,
                        ],
                    )
                    .map_err(|io_err| PacketError::DeserializeIOError(io_err))?;

                    if let [RawPacketValue::int(entity_id), RawPacketValue::ubyte(gamemode), RawPacketValue::byte(dimension), RawPacketValue::ubyte(difficulty), RawPacketValue::ubyte(max_players), RawPacketValue::String(level_type), RawPacketValue::boolean(reduced_debug_info)] =
                        packet_fields.as_slice()
                    {
                        Ok(Packet::ServerJoinGame {
                            entity_id: *entity_id,
                            gamemode: *gamemode,
                            dimension: *dimension,
                            difficulty: *difficulty,
                            max_players: *max_players,
                            level_type: level_type.clone(),
                            reduced_debug_info: *reduced_debug_info,
                        })
                    } else {
                        Err(PacketError::MalformedPacket("Handshake packet was BRRRRed"))
                    }
                }
                PacketType::ServerChatPacket => {
                    let packet_fields = read_values_from_template(
                        &mut packet_data_cursor,
                        &[RawPacketValueType::String, RawPacketValueType::byte],
                    )
                    .map_err(|io_err| PacketError::DeserializeIOError(io_err))?;

                    if let [RawPacketValue::String(message), RawPacketValue::byte(position)] =
                        packet_fields.as_slice()
                    {
                        Ok(Packet::ServerChatPacket {
                            message: message.clone(),
                            position: *position,
                        })
                    } else {
                        Err(PacketError::MalformedPacket("Handshake packet was BRRRRed"))
                    }
                }
                PacketType::ServerWorldTimeUpdate => {
                    let packet_fields = read_values_from_template(
                        &mut packet_data_cursor,
                        &[RawPacketValueType::long, RawPacketValueType::long],
                    )
                    .map_err(|io_err| PacketError::DeserializeIOError(io_err))?;

                    if let [RawPacketValue::long(age), RawPacketValue::long(time)] =
                        packet_fields.as_slice()
                    {
                        Ok(Packet::ServerWorldTimeUpdate {
                            age: *age,
                            time: *time,
                        })
                    } else {
                        Err(PacketError::MalformedPacket("Handshake packet was BRRRRed"))
                    }
                }
                PacketType::ServerPlayerPositionAndLook => {
                    let packet_fields = read_values_from_template(
                        &mut packet_data_cursor,
                        &[
                            RawPacketValueType::double,
                            RawPacketValueType::double,
                            RawPacketValueType::double,
                            RawPacketValueType::float,
                            RawPacketValueType::float,
                            RawPacketValueType::byte,
                            RawPacketValueType::varint,
                        ],
                    )
                    .map_err(|io_err| PacketError::DeserializeIOError(io_err))?;

                    if let [RawPacketValue::double(x), RawPacketValue::double(y), RawPacketValue::double(z), RawPacketValue::float(yaw), RawPacketValue::float(pitch), RawPacketValue::byte(flags), RawPacketValue::varint(teleportid)] =
                        packet_fields.as_slice()
                    {
                        Ok(Packet::ServerPlayerPositionAndLook {
                            x: *x,
                            y: *y,
                            z: *z,
                            yaw: *yaw,
                            pitch: *pitch,
                            flags: *flags,
                            teleportid: *teleportid,
                        })
                    } else {
                        Err(PacketError::MalformedPacket("Handshake packet was BRRRRed"))
                    }
                }
                (p) => {
                    dbg!(p);
                    Err(PacketError::MalformedPacket("Handshake packet was BRRRRed"))
                }
            }
        } else {
            Err(PacketError::UnknownPacketIdentifier {
                id: type_id,
                state: connection.state,
            })
        }
    }

    /// Takes the packet and serializes it for the server to receive
    pub fn serialize(self, connection: &MinecraftConnection) -> IOResult<Vec<u8>> {
        let mut buf = Vec::new();
        let my_id = self.ty().id();

        Ok(match self {
            Packet::ClientHandshake { host_address, port } => {
                write_packet_fields(
                    &mut buf,
                    my_id,
                    &[
                        RawPacketValue::varint(498), //TODO: CONST
                        RawPacketValue::String(host_address),
                        RawPacketValue::ushort(port),
                        RawPacketValue::varint(2),
                    ],
                    &connection.compression,
                )?;
                buf
            }
            Packet::ClientJoin { player_name } => {
                write_packet_fields(
                    &mut buf,
                    my_id,
                    &[RawPacketValue::String(player_name)],
                    &connection.compression,
                )?;
                buf
            }
            Packet::ClientKeepAlive { magic } => {
                write_packet_fields(
                    &mut buf,
                    my_id,
                    &[RawPacketValue::long(magic)],
                    &connection.compression,
                )?;
                buf
            }
            Packet::ClientPlayerPositionAndLook {
                x,
                y,
                z,
                yaw,
                pitch,
                onground,
            } => {
                write_packet_fields(
                    &mut buf,
                    my_id,
                    &[
                        RawPacketValue::double(x),
                        RawPacketValue::double(y),
                        RawPacketValue::double(z),
                        RawPacketValue::float(yaw),
                        RawPacketValue::float(pitch),
                        RawPacketValue::boolean(onground),
                    ],
                    &connection.compression,
                )?;
                buf
            }
            Packet::ClientPlayerPosition { x, y, z, onground } => {
                write_packet_fields(
                    &mut buf,
                    my_id,
                    &[
                        RawPacketValue::double(x),
                        RawPacketValue::double(y),
                        RawPacketValue::double(z),
                        RawPacketValue::boolean(onground),
                    ],
                    &connection.compression,
                )?;
                buf
            }
            Packet::ClientPlayerPositionAndLook {
                x,
                y,
                z,
                yaw,
                pitch,
                onground,
            } => {
                write_packet_fields(
                    &mut buf,
                    my_id,
                    &[
                        RawPacketValue::double(x),
                        RawPacketValue::double(y),
                        RawPacketValue::double(z),
                        RawPacketValue::float(yaw),
                        RawPacketValue::float(pitch),
                        RawPacketValue::boolean(onground),
                    ],
                    &connection.compression,
                )?;
                buf
            }
            Packet::ClientChat { message } => {
                write_packet_fields(
                    &mut buf,
                    my_id,
                    &[RawPacketValue::String(message)],
                    &connection.compression,
                )?;
                buf
            }
            Packet::ClientHeldItemChange { slot } => {
                write_packet_fields(
                    &mut buf,
                    my_id,
                    &[RawPacketValue::short(slot)],
                    &connection.compression,
                )?;
                buf
            }

            /*Packet::ServerCompressionLevelSet { .. } => {}
            Packet::ServerLoginSuccess { .. } => {}*/
            p => unimplemented!("this packet is not serializable: {:?}", p),
        })
    }
}

pub fn read_values_from_template(
    buf: &mut Cursor<Vec<u8>>,
    template: &[RawPacketValueType],
) -> IOResult<Vec<RawPacketValue>> {
    let mut out = Vec::with_capacity(template.len());
    for ty in template {
        out.push(ty.from_buf(buf)?);
        // println!("reading data {:X?}", &*buf);
    }

    assert_eq!(
        buf.get_ref().len(),
        buf.position() as usize,
        "packet malformed (or packet template ({:?})) ",
        template
    );

    Ok(out)
}

pub fn write_packet_fields<W>(
    buf: &mut W,
    packet_type_id: i32,
    template: &[RawPacketValue],
    compression_state: &CompressionStatus,
) -> IOResult<()>
where
    W: Write,
{
    let mut temp_buf = Vec::new();
    let mut temp_cursor = Cursor::new(&mut temp_buf);

    temp_cursor.write_var_i32(packet_type_id)?;
    for ty in template {
        ty.serialize(&mut temp_cursor);
    }
    let mut extra = 0;
    if let Enabled(threshold) = compression_state {
        extra = 1;
        if temp_buf.len() as i32 > *threshold {
            let uncompressed_size = temp_buf.len();
            let mut new = Vec::new();
            new.write_var_i32(uncompressed_size as i32);
            let mut write = ZlibEncoder::new(io::Cursor::new(temp_buf), Compression::default());
            write.read_to_end(&mut new)?;
            temp_buf = new;
        }
    }
    buf.write_var_i32(temp_buf.len() as i32 + extra)?; // schreibt als ERSTES in den buffer
    if let Enabled(threshold) = compression_state {
        if *threshold > temp_buf.len() as i32 {
            buf.write_var_i32(0 as i32)?; // Wenn compression dann anderes packetspec und weil hier an ist aber das packet zu klein ist eine 0
        }
    }
    buf.write_all(&temp_buf)?; // hängt den rest dran

    Ok(())
}
