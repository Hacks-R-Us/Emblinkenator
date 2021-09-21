use crossbeam::channel::{Receiver, RecvError, Sender};
use log::{error, warn};

use crate::{devices::threaded_device::ThreadedDevice, frame::FrameData, id::DeviceId, pipeline::PipelineContext};

use super::{AuxiliaryDataDevice, AuxiliaryDataType};

#[derive(Debug)]
pub enum NoiseType {
    Perlin
}

pub struct NoiseAuxiliaryDataDevice {
    id: DeviceId,
    noise_type: NoiseType,
    frame_data_buffer: Option<Receiver<FrameData>>,
    pipeline_context_buffer: Option<Receiver<PipelineContext>>,
    data_output_buffer: Option<Sender<AuxiliaryDataType>>,
}

impl NoiseAuxiliaryDataDevice {
    pub fn new (id: DeviceId) -> Self {
        NoiseAuxiliaryDataDevice {
            id,
            noise_type: NoiseType::Perlin,
            frame_data_buffer: None,
            pipeline_context_buffer: None,
            data_output_buffer: None
        }
    }

    fn get_noise_for_frame (&self, frame_data: FrameData, pipeline_context: PipelineContext) -> AuxiliaryDataType {

        match self.noise_type {
            NoiseType::Perlin => {
                AuxiliaryDataType::U32Vec(vec![])
            }
        }
    }
}

impl AuxiliaryDataDevice for NoiseAuxiliaryDataDevice {
    fn recieve_frame_data_buffer(&mut self, buffer: Receiver<FrameData>) {
        self.frame_data_buffer.replace(buffer);
    }

    fn send_into_buffer(&mut self, buffer: Sender<AuxiliaryDataType>) {
        self.data_output_buffer.replace(buffer);
    }
}

impl ThreadedDevice for NoiseAuxiliaryDataDevice {
    fn run (&mut self) {
        let frame_data_buffer = self.frame_data_buffer.as_mut();
        if frame_data_buffer.is_none() {
            return
        }

        let pipeline_context_buffer = self.pipeline_context_buffer.as_mut();
        if pipeline_context_buffer.is_none() {
            return
        }

        let frame_data_buffer = frame_data_buffer.unwrap();
        let frame_data = frame_data_buffer.recv();

        let pipeline_context_buffer = pipeline_context_buffer.unwrap();
        let pipeline_context = pipeline_context_buffer.recv();

        if frame_data.is_err() {
            error!("Frame data buffer exists but is closed! (Noise Device {})", self.id.unprotect());
            return
        }
        let frame_data = frame_data.unwrap();

        if pipeline_context.is_err() {
            error!("Pipeline context buffer exists but is closed! (Noise Device {})", self.id.unprotect());
            return
        }
        let pipeline_context = pipeline_context.unwrap();

        let data = self.get_noise_for_frame(frame_data, pipeline_context);
        if let Some(data_output_buffer) = self.data_output_buffer.as_mut() {
            data_output_buffer.send(data).ok();
        }
    }
}
