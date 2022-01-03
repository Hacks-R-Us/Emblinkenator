
use log::{error, warn};
use serde::Deserialize;
use tokio::sync::broadcast::{Receiver, Sender, channel, error::TryRecvError};
use std::net::UdpSocket;

use crate::{devices::{manager::DeviceInput, threaded_device::ThreadedDevice}, id::DeviceId};

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
    data_buffer_sender: Sender<DeviceInput>,
    data_buffer_receiver: Receiver<DeviceInput>
}

impl UDPSender {
    pub fn new(id: DeviceId, config: UDPSenderConfig) -> UDPSender {
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap(); // TODO: Panics!

        let (sender, receiver) = channel(1);

        UDPSender {
            id,
            name: config.name,
            socket,
            address: format!("{}:{}", config.host, config.port),
            data_buffer_sender: sender,
            data_buffer_receiver: receiver
        }
    }
}

impl ThreadedDevice for UDPSender {
    fn run(&mut self) {
        match self.data_buffer_receiver.try_recv() {
            Err(err) => match err {
                TryRecvError::Lagged(missed) => warn!("UDP device lagged by {} frames! (UDP Device {})", missed, self.id.unprotect()),
                TryRecvError::Closed => error!("Data buffer exists but is closed! (UDP Device {})", self.id.unprotect()),
                TryRecvError::Empty => {}
            },
            Ok(msg) => {
                match msg {
                    DeviceInput::LEDData(frame) => {
                        let payload: Vec<u8> = frame.iter().flat_map(|l| l.flat_u8()).collect();
                        self.socket.send_to(&payload, &self.address).ok();
                    },
                    DeviceInput::FrameData(_) => error!("UDPSender {} has received FrameData and doesn't know what to do with it!", self.id),
                    DeviceInput::NextFrameData(_) => error!("UDPSender {} has received NextFrameData and doesn't know what to do with it!", self.id),
                }
            }
        }
    }

    fn get_inputs (&self) -> Vec<crate::devices::manager::DeviceInputType> {
        todo!()
    }

    fn get_outputs (&self) -> Vec<crate::devices::manager::DeviceOutputType> {
        todo!()
    }

    fn send_to_input (&self, index: usize) -> Result<Sender<DeviceInput>, crate::devices::threaded_device::ThreadedDeviceInputError> {
        todo!()
    }

    fn receive_output (&self, index: usize) -> Result<Receiver<crate::devices::manager::DeviceOutput>, crate::devices::threaded_device::ThreadedDeviceOutputError> {
        todo!()
    }
}
