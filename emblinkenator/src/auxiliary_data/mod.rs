use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};

use log::{error, warn};
use parking_lot::RwLock;
use tokio::sync::broadcast::Receiver;

use crate::devices::manager::DeviceManagerEvent;
use crate::{
    devices::manager::DeviceManager,
    id::{AnimationId, AuxiliaryId, DeviceId},
    state::ThreadedObject,
};

#[derive(Debug, Clone)]
pub enum AuxiliaryDataType {
    Empty,
    U32(u32),
    F32(f32),
    U32Vec(Vec<u32>),
    F32Vec(Vec<f32>),
    U32Vec2(Vec<Vec<u32>>),
    F32Vec2(Vec<Vec<f32>>),
    U32Vec3(Vec<Vec<Vec<u32>>>),
    F32Vec3(Vec<Vec<Vec<f32>>>),
    U32Vec4(Vec<Vec<Vec<Vec<u32>>>>),
    F32Vec4(Vec<Vec<Vec<Vec<f32>>>>),
}

#[derive(Debug, Clone)]
pub struct AuxiliaryData {
    pub data: AuxiliaryDataType,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum AuxiliaryDataTypeConsumer {
    Empty,
    U32,
    F32,
    U32Vec,
    F32Vec,
    U32Vec2,
    F32Vec2,
    U32Vec3,
    F32Vec3,
    U32Vec4,
    F32Vec4,
}

pub struct AuxiliaryDataManager {
    auxiliary_to_device: RwLock<HashMap<AuxiliaryId, DeviceId>>,
    animation_auxiliary_sources: RwLock<HashMap<AnimationId, Vec<AuxiliaryId>>>,
    auxiliary_data_buffers: RwLock<HashMap<AuxiliaryId, Receiver<AuxiliaryData>>>,
    auxiliary_data: RwLock<HashMap<AuxiliaryId, AuxiliaryData>>,
    subscribed_events: EventSubscribers,
    device_manager: Arc<RwLock<DeviceManager>>,
}

#[derive(Default)]
struct EventSubscribers {
    device_manager: Vec<crossbeam::channel::Receiver<DeviceManagerEvent>>,
}

impl AuxiliaryDataManager {
    pub fn new(device_manager: Arc<RwLock<DeviceManager>>) -> Self {
        AuxiliaryDataManager {
            device_manager,
            auxiliary_to_device: RwLock::new(HashMap::new()),
            animation_auxiliary_sources: RwLock::new(HashMap::new()),
            auxiliary_data_buffers: RwLock::new(HashMap::new()),
            auxiliary_data: RwLock::new(HashMap::new()),
            subscribed_events: EventSubscribers::default(),
        }
    }

    pub fn get_auxiliary_data(&self) -> HashMap<AuxiliaryId, AuxiliaryData> {
        self.auxiliary_data.read().clone()
    }

    pub fn get_animation_auxiliary_ids(&self) -> HashMap<AnimationId, Vec<AuxiliaryId>> {
        self.animation_auxiliary_sources.read().clone()
    }

    pub fn subscribe_to_device_manager_events(
        &mut self,
        event_receiver: crossbeam::channel::Receiver<DeviceManagerEvent>,
    ) {
        self.subscribed_events.device_manager.push(event_receiver);
    }
}

impl ThreadedObject for AuxiliaryDataManager {
    fn run(&mut self) {
        for (aux_id, data_buffer) in self.auxiliary_data_buffers.write().iter_mut() {
            match data_buffer.try_recv() {
                Ok(data) => {
                    self.auxiliary_data.write().insert(aux_id.clone(), data);
                }
                Err(err) => match err {
                    tokio::sync::broadcast::error::TryRecvError::Empty => {}
                    tokio::sync::broadcast::error::TryRecvError::Closed => {
                        error!(
                            "Data channel for auxiliary {:?} has been closed",
                            aux_id.clone()
                        )
                    }
                    tokio::sync::broadcast::error::TryRecvError::Lagged(messages) => {
                        warn!(
                            "Lagged behind auxiliary device {:?} by {} frames",
                            aux_id.clone(),
                            messages
                        )
                    }
                },
            }
        }

        for device_manager in self.subscribed_events.device_manager.iter_mut() {
            for event in device_manager.try_iter() {
                match event {
                    DeviceManagerEvent::DeviceAdded(device_id) => {
                        if let Some(device) =
                            self.device_manager.read().get_device(device_id.clone())
                        {
                            match device {}
                        }
                        let aux_id = AuxiliaryId::new();
                        self.auxiliary_to_device
                            .write()
                            .insert(aux_id.clone(), device_id.clone());
                    }
                    DeviceManagerEvent::DeviceRemoved(deviceId) => todo!(),
                }
            }
        }
    }
}

impl AuxiliaryData {
    pub fn new(data: AuxiliaryDataType, size: u64) -> Self {
        AuxiliaryData { data, size }
    }
}

pub fn aux_data_is_compatible(
    data: AuxiliaryDataType,
    consumer: AuxiliaryDataTypeConsumer,
) -> bool {
    match consumer {
        AuxiliaryDataTypeConsumer::Empty => matches!(data, AuxiliaryDataType::Empty),
        AuxiliaryDataTypeConsumer::U32 => matches!(data, AuxiliaryDataType::U32(_)),
        AuxiliaryDataTypeConsumer::F32 => matches!(data, AuxiliaryDataType::F32(_)),
        AuxiliaryDataTypeConsumer::U32Vec => matches!(data, AuxiliaryDataType::U32Vec(_)),
        AuxiliaryDataTypeConsumer::F32Vec => matches!(data, AuxiliaryDataType::F32Vec(_)),
        AuxiliaryDataTypeConsumer::U32Vec2 => matches!(data, AuxiliaryDataType::U32Vec2(_)),
        AuxiliaryDataTypeConsumer::F32Vec2 => matches!(data, AuxiliaryDataType::F32Vec2(_)),
        AuxiliaryDataTypeConsumer::U32Vec3 => matches!(data, AuxiliaryDataType::U32Vec3(_)),
        AuxiliaryDataTypeConsumer::F32Vec3 => matches!(data, AuxiliaryDataType::F32Vec3(_)),
        AuxiliaryDataTypeConsumer::U32Vec4 => matches!(data, AuxiliaryDataType::U32Vec4(_)),
        AuxiliaryDataTypeConsumer::F32Vec4 => matches!(data, AuxiliaryDataType::F32Vec4(_)),
    }
}

pub fn aux_data_to_consumer_type(data: AuxiliaryDataType) -> AuxiliaryDataTypeConsumer {
    match data {
        AuxiliaryDataType::Empty => AuxiliaryDataTypeConsumer::Empty,
        AuxiliaryDataType::U32(_) => AuxiliaryDataTypeConsumer::U32,
        AuxiliaryDataType::F32(_) => AuxiliaryDataTypeConsumer::F32,
        AuxiliaryDataType::U32Vec(_) => AuxiliaryDataTypeConsumer::U32Vec,
        AuxiliaryDataType::F32Vec(_) => AuxiliaryDataTypeConsumer::F32Vec,
        AuxiliaryDataType::U32Vec2(_) => AuxiliaryDataTypeConsumer::U32Vec2,
        AuxiliaryDataType::F32Vec2(_) => AuxiliaryDataTypeConsumer::F32Vec2,
        AuxiliaryDataType::U32Vec3(_) => AuxiliaryDataTypeConsumer::U32Vec3,
        AuxiliaryDataType::F32Vec3(_) => AuxiliaryDataTypeConsumer::F32Vec3,
        AuxiliaryDataType::U32Vec4(_) => AuxiliaryDataTypeConsumer::U32Vec4,
        AuxiliaryDataType::F32Vec4(_) => AuxiliaryDataTypeConsumer::F32Vec4,
    }
}
