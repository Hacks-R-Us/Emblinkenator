use tokio::sync::broadcast::Receiver;

use crate::led::LED;

use super::threaded_device::ThreadedDevice;

pub mod mqtt;
pub mod udp;

pub trait LEDDataOutput: ThreadedDevice {
    fn set_data_buffer(&mut self, receiver: Receiver<Vec<LED>>);
}
