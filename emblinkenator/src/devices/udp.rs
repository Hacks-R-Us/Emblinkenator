
use serde::Deserialize;
use std::net::UdpSocket;

use crate::{id::DeviceId, led::LED};

use super::manager::LEDDataOutput;

#[derive(Clone, Deserialize)]
pub struct UDPSenderConfig {
    name: String,
    host: String,
    port: u16,
}

impl UDPSenderConfig {
    pub fn new(
        name: String,
        host: String,
        port: u16,
        socket: UdpSocket
    ) -> Self {
        UDPSenderConfig {
            name,
            host,
            port,
        }
    }
}

pub struct UDPSender {
    pub id: DeviceId,
    name: String,
    socket: UdpSocket,
    address: String
}

impl UDPSender {
    pub fn new(id: DeviceId, config: UDPSenderConfig) -> UDPSender {
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap(); // TODO: Panics!
        UDPSender {
            id,
            name: config.name,
            socket,
            address: format!("{}:{}", config.host, config.port)
        }
    }
}

impl LEDDataOutput for UDPSender {
    fn on_frame(&self, frame: Vec<LED>) {
        let payload: Vec<u8> = frame.iter().flat_map(|l| l.flat_u8()).collect();
        self.socket.send_to(&payload, &self.address).ok();
    }
}
