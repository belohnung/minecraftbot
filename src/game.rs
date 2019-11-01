use std::io::Result;
use std::net::{SocketAddrV4, TcpStream};
#[derive(Debug, Copy, Clone)]
pub enum ConnectionState {
    None,
    Status,
    Login,
    Play,
}

pub struct MinecraftConnection {
    server_address: SocketAddrV4,
    socket: Option<TcpStream>,
    player_name: String,
    state: ConnectionState,
}

impl MinecraftConnection {
    pub fn new(server_address: SocketAddrV4, player_name: String) -> MinecraftConnection {
        MinecraftConnection {
            server_address,
            socket: None,
            player_name,
            state: ConnectionState::None,
        }
    }

    pub fn connect(&mut self) -> Result<()> {
        self.socket = Some(TcpStream::connect(&self.server_address)?);
        Ok(())
    }

    pub fn ping(&mut self) -> Result<String> {
        unimplemented!();
        Ok("no".to_owned())
    }

    pub fn login(&mut self) {}
}
#[derive(Copy, Clone)]
pub struct Entity {
    pub entityid: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
}
