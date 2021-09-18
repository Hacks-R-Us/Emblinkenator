
use log::{error, warn};
use serde::Deserialize;
use tokio::sync::broadcast::{Receiver, error::TryRecvError};
use std::net::UdpSocket;

use crate::{id::DeviceId, led::LED};

use super::{manager::LEDDataOutput, threaded_device::ThreadedDevice};

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
        _socket: UdpSocket
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
    address: String,
    data_buffer: Option<Receiver<Vec<LED>>>
}

impl UDPSender {
    pub fn new(id: DeviceId, config: UDPSenderConfig) -> UDPSender {
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap(); // TODO: Panics!
        UDPSender {
            id,
            name: config.name,
            socket,
            address: format!("{}:{}", config.host, config.port),
            data_buffer: None
        }
    }
}

impl LEDDataOutput for UDPSender {
    fn set_data_buffer(&mut self, receiver: Receiver<Vec<LED>>) {
        self.data_buffer.replace(receiver);
    }
}

impl ThreadedDevice for UDPSender {
    fn run(&mut self) {
        if let Some(buffer) = &mut self.data_buffer {
            match buffer.try_recv() {
                Err(err) => match err {
                    TryRecvError::Lagged(missed) => warn!("MQTT device lagged by {} frames! (MQTT Device {})", missed, self.id.unprotect()),
                    TryRecvError::Closed => error!("Data buffer exists but is closed! (MQTT Device {})", self.id.unprotect()),
                    TryRecvError::Empty => {}
                },
                Ok(frame) => {
                    let payload: Vec<u8> = frame.iter().flat_map(|l| l.flat_u8()).collect();
                    self.socket.send_to(&payload, &self.address).ok();
                }
            }
        }
    }
}
