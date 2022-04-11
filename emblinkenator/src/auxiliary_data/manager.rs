use std::collections::HashMap;

use log::{debug, error, warn};
use parking_lot::RwLock;
use tokio::sync::broadcast::{channel, Receiver};

use crate::devices::auxiliary_data::AuxDeviceData;
use crate::state::WantsDeviceState;
use crate::{
    id::{AnimationId, AuxiliaryId, DeviceId},
    state::ThreadedObject,
};

use super::{AuxiliaryData, AuxiliaryDataType, AuxiliaryDataTypeConsumer};

#[derive(Debug)]
pub enum AddAuxiliaryError {
    AuxiliaryExists(AuxiliaryId),
}

pub struct AuxiliaryDataManager {
    animation_auxiliary_sources: RwLock<HashMap<AnimationId, Vec<AuxiliaryId>>>,
    auxiliary_data_buffers: RwLock<HashMap<DeviceId, Receiver<AuxDeviceData>>>,
    auxiliary_data: RwLock<HashMap<AuxiliaryId, AuxiliaryData>>,
}

impl AuxiliaryDataManager {
    pub fn new() -> Self {
        AuxiliaryDataManager {
            animation_auxiliary_sources: RwLock::new(HashMap::new()),
            auxiliary_data_buffers: RwLock::new(HashMap::new()),
            auxiliary_data: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_auxiliary(
        &self,
        aux_id: AuxiliaryId,
        aux_type: AuxiliaryDataTypeConsumer,
        params: AuxiliaryConfigParams,
    ) -> Result<(), AddAuxiliaryError> {
        let mut auxiliaries = self.auxiliary_data.write();
        if auxiliaries.contains_key(&aux_id) {
            return Err(AddAuxiliaryError::AuxiliaryExists(aux_id));
        }

        let mut default_value = aux_type.default_aux_value();
        match params {
            AuxiliaryConfigParams::Empty => {}
            AuxiliaryConfigParams::F32 {
                initial_value,
                min_value,
                max_value,
            } => {
                if let AuxiliaryDataType::F32(val) = &mut default_value {
                    *val = initial_value
                }
            }
            AuxiliaryConfigParams::F32Vec => {}
            AuxiliaryConfigParams::F32Vec2 => {}
            AuxiliaryConfigParams::F32Vec3 => {}
            AuxiliaryConfigParams::F32Vec4 => {}
        }
        let size = default_value.get_number_of_values();

        auxiliaries.insert(
            aux_id,
            AuxiliaryData {
                data: default_value,
                size,
            },
        );

        Ok(())
    }

    pub fn get_available_auxiliaries(&self) -> Vec<AuxiliaryId> {
        self.auxiliary_data.read().keys().cloned().collect()
    }

    pub fn get_auxiliary_data(&self) -> HashMap<AuxiliaryId, AuxiliaryData> {
        self.auxiliary_data.read().clone()
    }

    pub fn get_animation_auxiliary_ids(&self) -> HashMap<AnimationId, Vec<AuxiliaryId>> {
        self.animation_auxiliary_sources.read().clone()
    }

    pub fn set_animation_auxiliary_sources_to(
        &self,
        animation_id: AnimationId,
        sources: Vec<AuxiliaryId>,
    ) {
        // TODO: Validate all sources exist
        // TODO: Validate vec is the correct length
        self.animation_auxiliary_sources
            .write()
            .insert(animation_id, sources);
    }

    fn read_aux_data_from(&mut self, device_id: DeviceId, receiver: Receiver<AuxDeviceData>) {
        self.auxiliary_data_buffers
            .write()
            .insert(device_id, receiver);
    }
}

impl ThreadedObject for AuxiliaryDataManager {
    fn tick(&mut self) {
        for (aux_id, data_buffer) in self.auxiliary_data_buffers.write().iter_mut() {
            match data_buffer.try_recv() {
                Ok(data) => {
                    debug!("Received aux data from {}", aux_id);
                    // TODO: If size has changed, we need to recreate the auxiliary
                    let size = data.data.get_number_of_values();
                    if !self.auxiliary_data.read().contains_key(&data.aux_id) {
                        debug!("Recieved data for auxiliary {} which doesn't exist", aux_id);
                        continue;
                    }
                    self.auxiliary_data.write().insert(
                        data.aux_id.clone(),
                        AuxiliaryData {
                            data: data.data,
                            size,
                        },
                    );
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
    }
}

impl WantsDeviceState for AuxiliaryDataManager {
    fn on_device_added(&mut self, state: &crate::state::EmblinkenatorState, device_id: DeviceId) {
        if let Some(device) = state.get_device(&device_id) {
            match &mut *device.write() {
                crate::devices::manager::ThreadedDeviceType::LEDDataOutput(_) => {} // Nothing to do
                crate::devices::manager::ThreadedDeviceType::AuxiliaryData(aux_device) => {
                    let (sender, receiver) = channel(1);
                    aux_device.send_into_buffer(sender);
                    self.read_aux_data_from(device_id, receiver);
                }
            }
        }
    }
}

pub enum AuxiliaryConfigParams {
    Empty,
    F32 {
        initial_value: f32,
        min_value: f32,
        max_value: f32,
    },
    F32Vec,
    F32Vec2,
    F32Vec3,
    F32Vec4,
}

impl AuxiliaryConfigParams {
    fn is_compatible(&self, aux_type: AuxiliaryDataTypeConsumer) -> bool {
        match self {
            AuxiliaryConfigParams::Empty => matches!(aux_type, AuxiliaryDataTypeConsumer::Empty),
            AuxiliaryConfigParams::F32 {
                initial_value,
                min_value,
                max_value,
            } => matches!(aux_type, AuxiliaryDataTypeConsumer::F32),
            AuxiliaryConfigParams::F32Vec => {
                matches!(aux_type, AuxiliaryDataTypeConsumer::F32Vec)
            }
            AuxiliaryConfigParams::F32Vec2 => {
                matches!(aux_type, AuxiliaryDataTypeConsumer::F32Vec2)
            }
            AuxiliaryConfigParams::F32Vec3 => {
                matches!(aux_type, AuxiliaryDataTypeConsumer::F32Vec3)
            }
            AuxiliaryConfigParams::F32Vec4 => {
                matches!(aux_type, AuxiliaryDataTypeConsumer::F32Vec4)
            }
        }
    }
}
