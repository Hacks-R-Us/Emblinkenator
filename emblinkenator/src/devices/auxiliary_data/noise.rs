use crossbeam::channel::{Receiver, Sender};
use log::error;
use serde::Deserialize;
use noise::NoiseFn;

use crate::{auxiliary_data::AuxiliaryDataType, devices::threaded_device::ThreadedDevice, frame::FrameData, id::DeviceId, pipeline::PipelineContext};

use super::AuxiliaryDataDevice;

#[derive(Debug, Deserialize, Clone)]
pub enum NoiseType {
    Perlin
}

pub enum NoiseFunction {
    Perlin(noise::Perlin)
}

#[derive(Deserialize, Clone)]
pub struct NoiseAuxiliaryConfig {
    noise_type: NoiseType
}

pub struct NoiseAuxiliaryDataDevice {
    id: DeviceId,
    noise_function: NoiseFunction,
    frame_data_buffer: Option<Receiver<FrameData>>,
    pipeline_context_buffer: Option<Receiver<PipelineContext>>,
    data_output_buffer: Option<Sender<AuxiliaryDataType>>,
}

impl NoiseAuxiliaryDataDevice {
    pub fn new (id: DeviceId, config: NoiseAuxiliaryConfig) -> Self {
        let noise_function: NoiseFunction = match config.noise_type {
            NoiseType::Perlin => {
                NoiseFunction::Perlin(noise::Perlin::new())
            }
        };

        NoiseAuxiliaryDataDevice {
            id,
            noise_function,
            frame_data_buffer: None,
            pipeline_context_buffer: None,
            data_output_buffer: None
        }
    }

    fn get_noise_for_frame (&self, frame_data: FrameData) -> AuxiliaryDataType {
        let time_point = frame_data.frame / frame_data.frame_rate;

        match self.noise_function {
            NoiseFunction::Perlin(perlin) => {
                let mut res: Vec<Vec<Vec<f32>>> = vec![];

                for x in 0..1000 {
                    let mut y_vec: Vec<Vec<f32>> = vec![];
                    for y in 0..1000 {
                        let mut z_vec: Vec<f32> = vec![];
                        for z in 0..1000 {
                            z_vec.push(perlin.get([f64::from(x), f64::from(y), f64::from(z), f64::from(time_point)]) as _);
                        }
                        y_vec.push(z_vec);
                    }
                    res.push(y_vec);
                }

                AuxiliaryDataType::F32Vec3(res)
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

        let data = self.get_noise_for_frame(frame_data);
        if let Some(data_output_buffer) = self.data_output_buffer.as_mut() {
            data_output_buffer.send(data).ok();
        }
    }

    fn get_inputs (&self) -> Vec<crate::devices::manager::DeviceInputType> {
        todo!()
    }

    fn get_outputs (&self) -> Vec<crate::devices::manager::DeviceOutputType> {
        todo!()
    }

    fn send_to_input (&self, _index: usize) -> Result<tokio::sync::broadcast::Sender<crate::devices::manager::DeviceInput>, crate::devices::threaded_device::ThreadedDeviceInputError> {
        todo!()
    }

    fn receive_output (&self, _index: usize) -> Result<tokio::sync::broadcast::Receiver<crate::devices::manager::DeviceOutput>, crate::devices::threaded_device::ThreadedDeviceOutputError> {
        todo!()
    }
}
