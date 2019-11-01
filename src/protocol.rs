use crate::game::CompressionStatus::Enabled;
use crate::game::{CompressionStatus, ConnectionState, MinecraftConnection};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use err_derive::Error;
use mc_varint::{VarIntRead, VarIntWrite};
use std::borrow::BorrowMut;
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
    #[error(display = "packet not implemented: ID 0x{:02X}", id)]
    UnknownPacketIdentifier { id: i32 },
    #[error(display = "malformed packet: {}", 0)]
    MalformedPacket(&'static str),
    #[error(display = "i/o error while deserializing packet: {:?}", 0)]
    DeserializeIOError(std::io::Error),
    #[error(display = "du bastard")]
    SockySockyNoBlocky,
}

#[derive(Debug, Clone)]
pub enum Packet {
    ///////// C -> S (serverbound)
    // Login state
    ClientHandshake {
        host_address: String,
        port: u16,
    },
    ClientJoin {
        player_name: String,
    },

    // Status state
    // ..

    // Play state
    ClientKeepAlive {
        magic: i32,
    },
    ClientPlayerPosition {
        x: f64,
        y: f64,
        z: f64,
        onground: bool,
    },
    ClientPlayerLook {
        yaw: f32,
        pitch: f32,
        onground: bool,
    },
    ClientPlayerPositionAndLook {
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        pitch: f32,
        onground: bool,
    },
    ///////// S -> C (clientbound)
    // Login state
    ServerCompressionLevelSet {
        compression_level: i32,
    },
    ServerLoginSuccess {
        uuid: String,
        name: String,
    },
    // Status state
    // ..

    // Play state
    ServerKeepAlive {
        magic: i32,
    },
    ServerWorldTimeUpdate {
        age: i64,
        time: i64,
    },
    ServerChatPacket {
        message: String,
        position: i8,
    },
    ServerJoinGame {
        entity_id: i32,
        gamemode: u8,
        dimension: i8,
        difficulty: u8,
        max_players: u8,
        level_type: String,
        reduced_debug_info: bool,
    },
    ServerPlayerPositionAndLook {
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        pitch: f32,
        flags: i8,
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
        let mut packet_type_id = 0;
        let mut packet_data_cursor = Cursor::new(packet.clone());
        match connection.compression {
            CompressionStatus::Enabled(compression_level) => {
                let datalength = packet_cursor
                    .read_var_i32()
                    .map_err(|e| PacketError::DeserializeIOError(e))?;
                if datalength > 0 {
                    println!("oops");
                } else {
                    packet_type_id = packet_cursor
                        .read_var_i32()
                        .map_err(|e| PacketError::DeserializeIOError(e))?;
                    packet_data_cursor = Cursor::new(
                        packet
                            .as_slice()
                            .split_at(packet_cursor.position() as usize)
                            .1
                            .to_vec(),
                    );
                }
            }
            CompressionStatus::None => {
                packet_type_id = packet_cursor
                    .read_var_i32()
                    .map_err(|e| PacketError::DeserializeIOError(e))?;
                packet_data_cursor.set_position(packet_cursor.position());
            }
        }
        //println!("len: {:?}  data: {:X?}", packet_len, packet_data.clone());

        println!(
            "typeid: 0x{:02X} len: {:?}  data: {:02X?} CompressionStatus: {:?} PlayState: {:?}",
            packet_type_id,
            packet_len,
            packet_data_cursor,
            &connection.compression,
            &connection.state
        );
        match connection.state {
            ConnectionState::Login => match packet_type_id {
                0x02 => {
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

                0x03 => {
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

                id => Err(PacketError::UnknownPacketIdentifier { id }),
            },
            ConnectionState::Play => match packet_type_id {
                0x00 => {
                    let packet_fields = read_values_from_template(
                        &mut packet_data_cursor,
                        &[RawPacketValueType::varint],
                    )
                    .map_err(|io_err| PacketError::DeserializeIOError(io_err))?;

                    if let [RawPacketValue::varint(magic)] = packet_fields.as_slice() {
                        Ok(Packet::ServerKeepAlive { magic: *magic })
                    } else {
                        Err(PacketError::MalformedPacket("KeepAlive packet was BRRRRed"))
                    }
                }
                0x01 => {
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
                0x02 => {
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
                0x03 => {
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
                0x08 => {
                    let packet_fields = read_values_from_template(
                        &mut packet_data_cursor,
                        &[
                            RawPacketValueType::double,
                            RawPacketValueType::double,
                            RawPacketValueType::double,
                            RawPacketValueType::float,
                            RawPacketValueType::float,
                            RawPacketValueType::byte,
                        ],
                    )
                    .map_err(|io_err| PacketError::DeserializeIOError(io_err))?;

                    if let [RawPacketValue::double(x), RawPacketValue::double(y), RawPacketValue::double(z), RawPacketValue::float(yaw), RawPacketValue::float(pitch), RawPacketValue::byte(flags)] =
                        packet_fields.as_slice()
                    {
                        Ok(Packet::ServerPlayerPositionAndLook {
                            x: *x,
                            y: *y,
                            z: *z,
                            yaw: *yaw,
                            pitch: *pitch,
                            flags: *flags,
                        })
                    } else {
                        Err(PacketError::MalformedPacket("Handshake packet was BRRRRed"))
                    }
                }
                id => Err(PacketError::UnknownPacketIdentifier { id }),
            },
            _ => unimplemented!("state"),
        }
    }

    pub fn packet_type_id(&self) -> i32 {
        match &self {
            Packet::ClientHandshake { .. } => 0x00,
            Packet::ClientJoin { .. } => 0x00,
            Packet::ClientKeepAlive { .. } => 0x00,
            Packet::ClientPlayerPosition { .. } => 0x4,
            Packet::ClientPlayerLook { .. } => 0x05,
            Packet::ClientPlayerPositionAndLook { .. } => 0x06,

            Packet::ServerPlayerPositionAndLook { .. } => 0x08,
            Packet::ServerCompressionLevelSet { .. } => 0x03,
            Packet::ServerWorldTimeUpdate { .. } => 0x03,
            Packet::ServerLoginSuccess { .. } => 0x02,
            Packet::ServerChatPacket { .. } => 0x02,
            Packet::ServerKeepAlive { .. } => 0x00,
            Packet::ServerJoinGame { .. } => 0x01,
        }
    }

    /// Takes the packet and serializes it for the server to receive
    pub fn serialize(self) -> IOResult<Vec<u8>> {
        let mut buf = Vec::new();
        let my_id = self.packet_type_id();

        Ok(match self {
            Packet::ClientHandshake { host_address, port } => {
                write_packet_fields(
                    &mut buf,
                    my_id,
                    &[
                        RawPacketValue::varint(47),
                        RawPacketValue::String(host_address),
                        RawPacketValue::ushort(port),
                        RawPacketValue::varint(2),
                    ],
                )?;
                buf
            }
            Packet::ClientJoin { player_name } => {
                write_packet_fields(&mut buf, my_id, &[RawPacketValue::String(player_name)])?;
                buf
            }
            Packet::ClientKeepAlive { magic } => {
                write_packet_fields(&mut buf, my_id, &[RawPacketValue::varint(magic)])?;
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

    buf.write_var_i32(temp_buf.len() as i32)?;
    buf.write_all(&temp_buf)?;

    Ok(())
}
