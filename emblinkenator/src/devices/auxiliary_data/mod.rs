pub mod noise;

use crossbeam::channel::{Receiver, Sender};

use crate::{auxiliary_data::AuxiliaryDataType, devices::threaded_device::ThreadedDevice, frame::FrameData};

pub trait AuxiliaryDataDevice: ThreadedDevice {
    fn recieve_frame_data_buffer(&mut self, buffer: Receiver<FrameData>);
    fn send_into_buffer(&mut self, buffer: Sender<AuxiliaryDataType>);
}
