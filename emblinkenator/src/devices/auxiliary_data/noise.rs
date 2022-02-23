use std::{convert::TryInto, time::Instant};

use log::{debug, warn};
use noise::NoiseFn;
use serde::Deserialize;

use crate::{
    auxiliary_data::{AuxDataF32Vec3, AuxDataF32Vec3Unchecked, AuxiliaryDataType},
    frame::FrameData,
    id::DeviceId,
};

use super::AuxiliaryDataDevice;

#[derive(Debug, Deserialize, Clone)]
pub enum NoiseType {
    Perlin,
}

pub enum NoiseFunction {
    Perlin(noise::Perlin),
}

#[derive(Deserialize, Clone)]
pub struct NoiseAuxiliaryConfig {
    noise_type: NoiseType,
}

pub struct NoiseAuxiliaryDataDevice {
    id: DeviceId,
    noise_function: NoiseFunction,
    next_frame_data_buffer: Option<tokio::sync::broadcast::Receiver<FrameData>>,
    data_output_buffer: Option<tokio::sync::broadcast::Sender<AuxiliaryDataType>>,
}

impl NoiseAuxiliaryDataDevice {
    pub fn new(id: DeviceId, config: NoiseAuxiliaryConfig) -> Self {
        let noise_function: NoiseFunction = match config.noise_type {
            NoiseType::Perlin => NoiseFunction::Perlin(noise::Perlin::new()),
        };

        NoiseAuxiliaryDataDevice {
            id,
            noise_function,
            next_frame_data_buffer: None,
            data_output_buffer: None,
        }
    }

    fn get_noise_for_frame(&self, frame_data: FrameData) -> AuxiliaryDataType {
        let time_point = frame_data.frame / frame_data.frame_rate;

        match self.noise_function {
            NoiseFunction::Perlin(perlin) => {
                let mut res: Vec<Vec<Vec<f32>>> = vec![];

                let start = Instant::now();

                for x in 0..10 {
                    let mut y_vec: Vec<Vec<f32>> = vec![];
                    for y in 0..10 {
                        let mut z_vec: Vec<f32> = vec![];
                        for z in 0..10 {
                            z_vec.push(perlin.get([
                                f64::from(x),
                                f64::from(y),
                                f64::from(z),
                                f64::from(time_point),
                            ]) as _);
                        }
                        y_vec.push(z_vec);
                    }
                    res.push(y_vec);
                }

                let elapsed_time = Instant::now().duration_since(start).as_millis();

                debug!(
                    "Calculated {} noise values in {}ms",
                    u64::pow(10, 3),
                    elapsed_time
                );

                let unchecked_data = AuxDataF32Vec3Unchecked {
                    data: res,
                    size_dimension_1: 10,
                    size_dimension_2: 10,
                    size_dimension_3: 10,
                };

                AuxiliaryDataType::F32Vec3(
                    unchecked_data
                        .try_into()
                        .expect("Noise data should conform to requirements"),
                )
            }
        }
    }
}

impl AuxiliaryDataDevice for NoiseAuxiliaryDataDevice {
    fn receive_next_frame_data_buffer(
        &mut self,
        buffer: tokio::sync::broadcast::Receiver<FrameData>,
    ) {
        self.next_frame_data_buffer.replace(buffer);
    }

    fn send_into_buffer(&mut self, buffer: tokio::sync::broadcast::Sender<AuxiliaryDataType>) {
        self.data_output_buffer.replace(buffer);
    }

    fn tick(&mut self) {
        let next_frame_data_buffer = self.next_frame_data_buffer.as_mut();
        if next_frame_data_buffer.is_none() {
            return;
        }

        let next_frame_data_buffer = next_frame_data_buffer.unwrap();
        match next_frame_data_buffer.try_recv() {
            Ok(next_frame_data) => {
                let data = self.get_noise_for_frame(next_frame_data);
                if let Some(data_output_buffer) = self.data_output_buffer.as_mut() {
                    data_output_buffer.send(data).ok();
                }
            }
            Err(err) => match err {
                tokio::sync::broadcast::error::TryRecvError::Empty => {}
                tokio::sync::broadcast::error::TryRecvError::Closed => {
                    debug!(
                        "Noise auxiliary {} had next frame data buffer closed",
                        self.id.unprotect()
                    );
                    self.next_frame_data_buffer.take();
                }
                tokio::sync::broadcast::error::TryRecvError::Lagged(num) => warn!(
                    "Noise Auxiliary device {} lagged by {} frames",
                    self.id.unprotect(),
                    num
                ),
            },
        }
    }
}
