use log::{error, warn};
use serde::Deserialize;
use std::net::UdpSocket;
use tokio::sync::broadcast::{error::TryRecvError, Receiver};

use crate::{frame_resolver::LEDFrame, id::DeviceId};

use super::LEDOutputDevice;

#[derive(Clone, Deserialize)]
pub struct UDPSenderConfig {
    name: String,
    host: String,
    port: u16,
}

impl UDPSenderConfig {
    pub fn new(name: String, host: String, port: u16, _socket: UdpSocket) -> Self {
        UDPSenderConfig { name, host, port }
    }
}

pub struct UDPSender {
    pub id: DeviceId,
    name: String,
    socket: UdpSocket,
    address: String,
    data_buffer_receiver: Option<Receiver<LEDFrame>>,
}

impl UDPSender {
    pub fn new(id: DeviceId, config: UDPSenderConfig) -> UDPSender {
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap(); // TODO: Panics!

        UDPSender {
            id,
            name: config.name,
            socket,
            address: format!("{}:{}", config.host, config.port),
            data_buffer_receiver: None,
        }
    }
}

impl LEDOutputDevice for UDPSender {
    fn tick(&mut self) {
        if let Some(data_buffer_receiver) = &mut self.data_buffer_receiver {
            match data_buffer_receiver.try_recv() {
                Err(err) => match err {
                    TryRecvError::Lagged(missed) => warn!("UDP device lagged by {} frames! (UDP Device {})", missed, self.id.unprotect()),
                    TryRecvError::Closed => error!("Data buffer exists but is closed! (UDP Device {})", self.id.unprotect()), // TODO: Remove buffer
                    TryRecvError::Empty => {}
                },
                Ok(frame) => {
                    let payload: Vec<u8> = frame.iter().flat_map(|l| l.flat_u8()).collect();
                    self.socket.send_to(&payload, &self.address).ok();
                }
            }
        }
    }

    fn receive_data_from(&mut self, buffer: Receiver<crate::frame_resolver::LEDFrame>) {
        self.data_buffer_receiver.replace(buffer);
    }
}
