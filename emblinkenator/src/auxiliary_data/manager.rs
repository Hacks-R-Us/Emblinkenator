use std::collections::HashMap;

use log::{debug, error, warn};
use parking_lot::RwLock;
use tokio::sync::broadcast::{channel, Receiver};

use crate::state::WantsDeviceState;
use crate::{
    id::{AnimationId, AuxiliaryId, DeviceId},
    state::ThreadedObject,
};

use super::{AuxiliaryData, AuxiliaryDataType};

pub struct AuxiliaryDataManager {
    auxiliary_to_device: RwLock<HashMap<AuxiliaryId, DeviceId>>,
    animation_auxiliary_sources: RwLock<HashMap<AnimationId, Vec<AuxiliaryId>>>,
    auxiliary_data_buffers: RwLock<HashMap<AuxiliaryId, Receiver<AuxiliaryDataType>>>,
    auxiliary_data: RwLock<HashMap<AuxiliaryId, AuxiliaryData>>,
}

impl AuxiliaryDataManager {
    pub fn new() -> Self {
        AuxiliaryDataManager {
            auxiliary_to_device: RwLock::new(HashMap::new()),
            animation_auxiliary_sources: RwLock::new(HashMap::new()),
            auxiliary_data_buffers: RwLock::new(HashMap::new()),
            auxiliary_data: RwLock::new(HashMap::new()),
        }
    }

    pub fn get_available_auxiliaries(&self) -> Vec<AuxiliaryId> {
        self.auxiliary_to_device.read().keys().cloned().collect()
    }

    // TODO: Remove
    pub fn hack_get_device_of_auxiliary(&self, aux_id: &AuxiliaryId) -> Option<DeviceId> {
        self.auxiliary_to_device.read().get(aux_id).cloned()
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

    fn read_aux_data_from(
        &mut self,
        auxiliary_id: AuxiliaryId,
        receiver: Receiver<AuxiliaryDataType>,
    ) {
        self.auxiliary_data_buffers
            .write()
            .insert(auxiliary_id, receiver);
    }
}

impl ThreadedObject for AuxiliaryDataManager {
    fn tick(&mut self) {
        for (aux_id, data_buffer) in self.auxiliary_data_buffers.write().iter_mut() {
            match data_buffer.try_recv() {
                Ok(data) => {
                    debug!("Received aux data from {}", aux_id);
                    // TODO: If size has changed, we need to recreate the auxiliary
                    let size = data.get_number_of_values();
                    self.auxiliary_data
                        .write()
                        .insert(aux_id.clone(), AuxiliaryData { data, size });
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
                    let aux_id = AuxiliaryId::new();
                    self.auxiliary_to_device
                        .write()
                        .insert(aux_id.clone(), device_id.clone());
                    let (sender, receiver) = channel(1);
                    aux_device.send_into_buffer(sender);
                    self.read_aux_data_from(aux_id, receiver);
                }
            }
        }
    }
}
