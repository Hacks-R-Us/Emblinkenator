use std::collections::HashMap;
use serde::{Deserialize};

use log::{error, warn};
use parking_lot::RwLock;
use tokio::sync::broadcast::Receiver;

use crate::{id::{AnimationId, AuxiliaryId}, state::ThreadedObject};

#[derive(Debug, Clone)]
pub enum AuxiliaryDataType {
    Empty,
    U32(u32),
    F32(f32),
    U32Vec(Vec<u32>),
    F32Vec(Vec<f32>),
    U32Vec2(Vec<Vec<u32>>),
    F32Vec2(Vec<Vec<f32>>)
}

#[derive(Debug, Clone, Deserialize)]
pub enum AuxiliaryDataTypeConsumer {
    U32,
    F32,
    U32Vec,
    F32Vec,
    U32Vec2,
    F32Vec2
}

pub struct AuxiliaryDataManager {
    animation_auxiliary_sources: RwLock<HashMap<AnimationId, Vec<AuxiliaryId>>>,
    auxiliary_data_buffers: RwLock<HashMap<AuxiliaryId, Receiver<AuxiliaryDataType>>>,
    auxiliary_data: RwLock<HashMap<AuxiliaryId, AuxiliaryDataType>>
}

impl AuxiliaryDataManager {
    pub fn new () -> Self {
        AuxiliaryDataManager {
            animation_auxiliary_sources: RwLock::new(HashMap::new()),
            auxiliary_data_buffers: RwLock::new(HashMap::new()),
            auxiliary_data: RwLock::new(HashMap::new())
        }
    }

    pub fn get_auxiliary_data (&self) -> HashMap<AnimationId, Vec<AuxiliaryDataType>> {
        let mut result = HashMap::new();
        for (animation_id, sources) in self.animation_auxiliary_sources.read().iter() {
            let mut data_vec: Vec<AuxiliaryDataType> = vec![];
            for aux_id in sources {
                if let Some (data) = self.auxiliary_data.read().get(aux_id) {
                    data_vec.push(data.clone());
                } else {
                    data_vec.push(AuxiliaryDataType::Empty);
                }
            }
            result.insert(animation_id.clone(), data_vec);
        }

        result
    }
}

impl ThreadedObject for AuxiliaryDataManager {
    fn run(&mut self) {
        for (aux_id, data_buffer) in self.auxiliary_data_buffers.write().iter_mut() {
            match data_buffer.try_recv() {
                Ok(data) => {
                    self.auxiliary_data.write().insert(aux_id.clone(), data);
                },
                Err(err) => match err {
                    tokio::sync::broadcast::error::TryRecvError::Empty => {},
                    tokio::sync::broadcast::error::TryRecvError::Closed => {
                        error!("Data channel for auxiliary {:?} has been closed", aux_id.clone())
                    },
                    tokio::sync::broadcast::error::TryRecvError::Lagged(messages) => {
                        warn!("Lagged behind auxiliary device {:?} by {} frames", aux_id.clone(), messages)
                    },
                },
            }
        }
    }
}

pub fn aux_data_is_compatible (data: AuxiliaryDataType, consumer: AuxiliaryDataTypeConsumer) -> bool {
    match consumer {
        AuxiliaryDataTypeConsumer::U32 => matches!(data, AuxiliaryDataType::U32(_)),
        AuxiliaryDataTypeConsumer::F32 => matches!(data, AuxiliaryDataType::F32(_)),
        AuxiliaryDataTypeConsumer::U32Vec => matches!(data, AuxiliaryDataType::U32Vec(_)),
        AuxiliaryDataTypeConsumer::F32Vec => matches!(data, AuxiliaryDataType::F32Vec(_)),
        AuxiliaryDataTypeConsumer::U32Vec2 => matches!(data, AuxiliaryDataType::U32Vec2(_)),
        AuxiliaryDataTypeConsumer::F32Vec2 => matches!(data, AuxiliaryDataType::F32Vec2(_)),
    }
}
