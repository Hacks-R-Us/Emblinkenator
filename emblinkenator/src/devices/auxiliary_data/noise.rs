use log::error;
use serde::Deserialize;
use noise::NoiseFn;

use crate::{auxiliary_data::{AuxiliaryDataType, AuxiliaryData}, frame::FrameData, id::DeviceId};

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
    next_frame_data_buffer: Option<crossbeam::channel::Receiver<FrameData>>,
    data_output_buffer: Option<tokio::sync::broadcast::Sender<AuxiliaryData>>,
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
            next_frame_data_buffer: None,
            data_output_buffer: None
        }
    }

    fn get_noise_for_frame (&self, frame_data: FrameData) -> AuxiliaryData {
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

                AuxiliaryData::new(AuxiliaryDataType::F32Vec3(res), u64::pow(1000, 3))
            }
        }
    }
}

impl AuxiliaryDataDevice for NoiseAuxiliaryDataDevice {
    fn receive_next_frame_data_buffer(&mut self, buffer: crossbeam::channel::Receiver<FrameData>) {
        self.next_frame_data_buffer.replace(buffer);
    }

    fn send_into_buffer(&mut self, buffer: tokio::sync::broadcast::Sender<AuxiliaryData>) {
        self.data_output_buffer.replace(buffer);
    }

    fn tick(&mut self) {
        let next_frame_data_buffer = self.next_frame_data_buffer.as_mut();
        if next_frame_data_buffer.is_none() {
            return
        }

        let next_frame_data_buffer = next_frame_data_buffer.unwrap();
        let next_frame_data = next_frame_data_buffer.recv();

        if next_frame_data.is_err() {
            error!("Frame data buffer exists but is closed! (Noise Device {})", self.id.unprotect());
            return
        }
        let next_frame_data = next_frame_data.unwrap();

        let data = self.get_noise_for_frame(next_frame_data);
        if let Some(data_output_buffer) = self.data_output_buffer.as_mut() {
            data_output_buffer.send(data).ok();
        }
    }

}
