use serde::Deserialize;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::Display;
use std::mem;
use strum_macros::EnumIter;

use log::{debug, error, warn};
use parking_lot::RwLock;
use tokio::sync::broadcast::{channel, Receiver};

use crate::state::WantsDeviceState;
use crate::{
    id::{AnimationId, AuxiliaryId, DeviceId},
    state::ThreadedObject,
};

#[derive(Debug, Clone)]
pub enum AuxiliaryDataType {
    Empty,
    U32(u32),
    F32(f32),
    U32Vec(AuxDataU32Vec),
    F32Vec(AuxDataF32Vec),
    U32Vec2(AuxDataU32Vec2),
    F32Vec2(AuxDataF32Vec2),
    U32Vec3(AuxDataU32Vec3),
    F32Vec3(AuxDataF32Vec3),
    U32Vec4(AuxDataU32Vec4),
    F32Vec4(AuxDataF32Vec4),
}

#[derive(Debug, Clone)]
pub struct AuxDataU32Vec {
    data: Vec<u32>,
    size_dimension_1: u32,
}

#[derive(Debug, Clone)]
pub struct AuxDataF32Vec {
    data: Vec<f32>,
    size_dimension_1: u32,
}

#[derive(Debug, Clone)]
pub struct AuxDataU32Vec2 {
    data: Vec<Vec<u32>>,
    size_dimension_1: u32,
    size_dimension_2: u32,
}

#[derive(Debug, Clone)]
pub struct AuxDataF32Vec2 {
    data: Vec<Vec<f32>>,
    size_dimension_1: u32,
    size_dimension_2: u32,
}

#[derive(Debug, Clone)]
pub struct AuxDataU32Vec3 {
    data: Vec<Vec<Vec<u32>>>,
    size_dimension_1: u32,
    size_dimension_2: u32,
    size_dimension_3: u32,
}

#[derive(Debug, Clone)]
pub struct AuxDataF32Vec3 {
    data: Vec<Vec<Vec<f32>>>,
    size_dimension_1: u32,
    size_dimension_2: u32,
    size_dimension_3: u32,
}

#[derive(Debug, Clone)]
pub struct AuxDataU32Vec4 {
    data: Vec<Vec<Vec<Vec<u32>>>>,
    size_dimension_1: u32,
    size_dimension_2: u32,
    size_dimension_3: u32,
    size_dimension_4: u32,
}

#[derive(Debug, Clone)]
pub struct AuxDataF32Vec4 {
    data: Vec<Vec<Vec<Vec<f32>>>>,
    size_dimension_1: u32,
    size_dimension_2: u32,
    size_dimension_3: u32,
    size_dimension_4: u32,
}

#[derive(Debug, Clone)]
pub struct AuxDataU32VecUnchecked {
    pub data: Vec<u32>,
    pub size_dimension_1: u32,
}

impl TryFrom<AuxDataU32VecUnchecked> for AuxDataU32Vec {
    type Error = AuxDataFromVecError;

    fn try_from(value: AuxDataU32VecUnchecked) -> Result<Self, Self::Error> {
        if value.data.len() != value.size_dimension_1 as usize {
            return Result::Err(AuxDataFromVecError::DimensionMismatch);
        }

        Ok(AuxDataU32Vec {
            data: value.data,
            size_dimension_1: value.size_dimension_1,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuxDataF32VecUnchecked {
    pub data: Vec<f32>,
    pub size_dimension_1: u32,
}

impl TryFrom<AuxDataF32VecUnchecked> for AuxDataF32Vec {
    type Error = AuxDataFromVecError;

    fn try_from(value: AuxDataF32VecUnchecked) -> Result<Self, Self::Error> {
        if value.data.len() != value.size_dimension_1 as usize {
            return Result::Err(AuxDataFromVecError::DimensionMismatch);
        }

        Ok(AuxDataF32Vec {
            data: value.data,
            size_dimension_1: value.size_dimension_1,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuxDataU32Vec2Unchecked {
    pub data: Vec<Vec<u32>>,
    pub size_dimension_1: u32,
    pub size_dimension_2: u32,
}

impl TryFrom<AuxDataU32Vec2Unchecked> for AuxDataU32Vec2 {
    type Error = AuxDataFromVecError;

    fn try_from(value: AuxDataU32Vec2Unchecked) -> Result<Self, Self::Error> {
        if value.data.len() != value.size_dimension_1 as usize {
            return Result::Err(AuxDataFromVecError::DimensionMismatch);
        }

        if value
            .data
            .iter()
            .any(|d| d.len() != value.size_dimension_2 as usize)
        {
            return Result::Err(AuxDataFromVecError::DimensionMismatch);
        }

        Ok(AuxDataU32Vec2 {
            data: value.data,
            size_dimension_1: value.size_dimension_1,
            size_dimension_2: value.size_dimension_2,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuxDataF32Vec2Unchecked {
    pub data: Vec<Vec<f32>>,
    pub size_dimension_1: u32,
    pub size_dimension_2: u32,
}

impl TryFrom<AuxDataF32Vec2Unchecked> for AuxDataF32Vec2 {
    type Error = AuxDataFromVecError;

    fn try_from(value: AuxDataF32Vec2Unchecked) -> Result<Self, Self::Error> {
        if value.data.len() != value.size_dimension_1 as usize {
            return Result::Err(AuxDataFromVecError::DimensionMismatch);
        }

        if value
            .data
            .iter()
            .any(|d| d.len() != value.size_dimension_2 as usize)
        {
            return Result::Err(AuxDataFromVecError::DimensionMismatch);
        }

        Ok(AuxDataF32Vec2 {
            data: value.data,
            size_dimension_1: value.size_dimension_1,
            size_dimension_2: value.size_dimension_2,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuxDataU32Vec3Unchecked {
    pub data: Vec<Vec<Vec<u32>>>,
    pub size_dimension_1: u32,
    pub size_dimension_2: u32,
    pub size_dimension_3: u32,
}

impl TryFrom<AuxDataU32Vec3Unchecked> for AuxDataU32Vec3 {
    type Error = AuxDataFromVecError;

    fn try_from(value: AuxDataU32Vec3Unchecked) -> Result<Self, Self::Error> {
        if value.data.len() != value.size_dimension_1 as usize {
            return Result::Err(AuxDataFromVecError::DimensionMismatch);
        }

        for val in value.data.iter() {
            if val.len() != value.size_dimension_2 as usize {
                return Result::Err(AuxDataFromVecError::DimensionMismatch);
            }

            if val
                .iter()
                .any(|d| d.len() != value.size_dimension_3 as usize)
            {
                return Result::Err(AuxDataFromVecError::DimensionMismatch);
            }
        }

        Ok(AuxDataU32Vec3 {
            data: value.data,
            size_dimension_1: value.size_dimension_1,
            size_dimension_2: value.size_dimension_2,
            size_dimension_3: value.size_dimension_3,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuxDataF32Vec3Unchecked {
    pub data: Vec<Vec<Vec<f32>>>,
    pub size_dimension_1: u32,
    pub size_dimension_2: u32,
    pub size_dimension_3: u32,
}

impl TryFrom<AuxDataF32Vec3Unchecked> for AuxDataF32Vec3 {
    type Error = AuxDataFromVecError;

    fn try_from(value: AuxDataF32Vec3Unchecked) -> Result<Self, Self::Error> {
        if value.data.len() != value.size_dimension_1 as usize {
            return Result::Err(AuxDataFromVecError::DimensionMismatch);
        }

        for val in value.data.iter() {
            if val.len() != value.size_dimension_2 as usize {
                return Result::Err(AuxDataFromVecError::DimensionMismatch);
            }

            if val
                .iter()
                .any(|d| d.len() != value.size_dimension_3 as usize)
            {
                return Result::Err(AuxDataFromVecError::DimensionMismatch);
            }
        }

        Ok(AuxDataF32Vec3 {
            data: value.data,
            size_dimension_1: value.size_dimension_1,
            size_dimension_2: value.size_dimension_2,
            size_dimension_3: value.size_dimension_3,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuxDataU32Vec4Unchecked {
    pub data: Vec<Vec<Vec<Vec<u32>>>>,
    pub size_dimension_1: u32,
    pub size_dimension_2: u32,
    pub size_dimension_3: u32,
    pub size_dimension_4: u32,
}

impl TryFrom<AuxDataU32Vec4Unchecked> for AuxDataU32Vec4 {
    type Error = AuxDataFromVecError;

    fn try_from(value: AuxDataU32Vec4Unchecked) -> Result<Self, Self::Error> {
        if value.data.len() != value.size_dimension_1 as usize {
            return Result::Err(AuxDataFromVecError::DimensionMismatch);
        }

        for val in value.data.iter() {
            if val.len() != value.size_dimension_2 as usize {
                return Result::Err(AuxDataFromVecError::DimensionMismatch);
            }

            for val2 in val.iter() {
                if val2.len() != value.size_dimension_3 as usize
                    || val2
                        .iter()
                        .any(|d| d.len() != value.size_dimension_4 as usize)
                {
                    return Result::Err(AuxDataFromVecError::DimensionMismatch);
                }
            }
        }

        Ok(AuxDataU32Vec4 {
            data: value.data,
            size_dimension_1: value.size_dimension_1,
            size_dimension_2: value.size_dimension_2,
            size_dimension_3: value.size_dimension_3,
            size_dimension_4: value.size_dimension_4,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuxDataF32Vec4Unchecked {
    pub data: Vec<Vec<Vec<Vec<f32>>>>,
    pub size_dimension_1: u32,
    pub size_dimension_2: u32,
    pub size_dimension_3: u32,
    pub size_dimension_4: u32,
}

impl TryFrom<AuxDataF32Vec4Unchecked> for AuxDataF32Vec4 {
    type Error = AuxDataFromVecError;

    fn try_from(value: AuxDataF32Vec4Unchecked) -> Result<Self, Self::Error> {
        if value.data.len() != value.size_dimension_1 as usize {
            return Result::Err(AuxDataFromVecError::DimensionMismatch);
        }

        for val in value.data.iter() {
            if val.len() != value.size_dimension_2 as usize {
                return Result::Err(AuxDataFromVecError::DimensionMismatch);
            }

            for val2 in val.iter() {
                if val2.len() != value.size_dimension_3 as usize
                    || val2
                        .iter()
                        .any(|d| d.len() != value.size_dimension_4 as usize)
                {
                    return Result::Err(AuxDataFromVecError::DimensionMismatch);
                }
            }
        }

        Ok(AuxDataF32Vec4 {
            data: value.data,
            size_dimension_1: value.size_dimension_1,
            size_dimension_2: value.size_dimension_2,
            size_dimension_3: value.size_dimension_3,
            size_dimension_4: value.size_dimension_4,
        })
    }
}

#[derive(Debug)]
pub enum AuxDataFromVecError {
    DimensionMismatch,
    TooBig,
}

#[derive(Debug, Clone)]
pub struct AuxiliaryData {
    pub data: AuxiliaryDataType,
    pub size: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash, EnumIter)]
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

impl AuxiliaryData {
    pub fn new(data: AuxiliaryDataType, size: u32) -> Self {
        AuxiliaryData { data, size }
    }
}

impl AuxiliaryDataType {
    pub fn to_data_buffer(&self) -> Vec<u8> {
        match self {
            AuxiliaryDataType::Empty => vec![],
            AuxiliaryDataType::U32(val) => val.to_be_bytes().to_vec(),
            AuxiliaryDataType::F32(val) => val.to_be_bytes().to_vec(),
            AuxiliaryDataType::U32Vec(val) => vec![
                (val.size_dimension_1).to_be_bytes().to_vec(),
                bytemuck::cast_slice(&val.data).to_vec(),
            ]
            .concat(),
            AuxiliaryDataType::F32Vec(val) => vec![
                (val.size_dimension_1).to_be_bytes().to_vec(),
                bytemuck::cast_slice(&val.data).to_vec(),
            ]
            .concat(),
            AuxiliaryDataType::U32Vec2(val) => vec![
                val.size_dimension_1.to_be_bytes().to_vec(),
                val.size_dimension_2.to_be_bytes().to_vec(),
                bytemuck::cast_slice(&val.data.iter().flatten().cloned().collect::<Vec<u32>>())
                    .to_vec(),
            ]
            .concat(),
            AuxiliaryDataType::F32Vec2(val) => vec![
                val.size_dimension_1.to_be_bytes().to_vec(),
                val.size_dimension_2.to_be_bytes().to_vec(),
                bytemuck::cast_slice(&val.data.iter().flatten().cloned().collect::<Vec<f32>>())
                    .to_vec(),
            ]
            .concat(),
            AuxiliaryDataType::U32Vec3(val) => vec![
                val.size_dimension_1.to_be_bytes().to_vec(),
                val.size_dimension_2.to_be_bytes().to_vec(),
                val.size_dimension_3.to_be_bytes().to_vec(),
                bytemuck::cast_slice(
                    &val.data
                        .iter()
                        .flatten()
                        .into_iter()
                        .flatten()
                        .cloned()
                        .collect::<Vec<u32>>(),
                )
                .to_vec(),
            ]
            .concat(),
            AuxiliaryDataType::F32Vec3(val) => vec![
                val.size_dimension_1.to_be_bytes().to_vec(),
                val.size_dimension_2.to_be_bytes().to_vec(),
                val.size_dimension_3.to_be_bytes().to_vec(),
                bytemuck::cast_slice(
                    &val.data
                        .iter()
                        .flatten()
                        .into_iter()
                        .flatten()
                        .cloned()
                        .collect::<Vec<f32>>(),
                )
                .to_vec(),
            ]
            .concat(),
            AuxiliaryDataType::U32Vec4(val) => vec![
                val.size_dimension_1.to_be_bytes().to_vec(),
                val.size_dimension_2.to_be_bytes().to_vec(),
                val.size_dimension_3.to_be_bytes().to_vec(),
                val.size_dimension_4.to_be_bytes().to_vec(),
                bytemuck::cast_slice(
                    &val.data
                        .iter()
                        .flatten()
                        .into_iter()
                        .flatten()
                        .into_iter()
                        .flatten()
                        .cloned()
                        .collect::<Vec<u32>>(),
                )
                .to_vec(),
            ]
            .concat(),
            AuxiliaryDataType::F32Vec4(val) => vec![
                val.size_dimension_1.to_be_bytes().to_vec(),
                val.size_dimension_2.to_be_bytes().to_vec(),
                val.size_dimension_3.to_be_bytes().to_vec(),
                val.size_dimension_4.to_be_bytes().to_vec(),
                bytemuck::cast_slice(
                    &val.data
                        .iter()
                        .flatten()
                        .into_iter()
                        .flatten()
                        .into_iter()
                        .flatten()
                        .cloned()
                        .collect::<Vec<f32>>(),
                )
                .to_vec(),
            ]
            .concat(),
        }
    }

    pub fn get_number_of_values(&self) -> u32 {
        match self {
            AuxiliaryDataType::Empty => 0,
            AuxiliaryDataType::U32(_) => 1,
            AuxiliaryDataType::F32(_) => 1,
            AuxiliaryDataType::U32Vec(val) => val.size_dimension_1 + 1,
            AuxiliaryDataType::F32Vec(val) => val.size_dimension_1 + 1,
            AuxiliaryDataType::U32Vec2(val) => val.size_dimension_1 * val.size_dimension_2 + 2,
            AuxiliaryDataType::F32Vec2(val) => val.size_dimension_1 * val.size_dimension_2 + 2,
            AuxiliaryDataType::U32Vec3(val) => {
                val.size_dimension_1 * val.size_dimension_2 * val.size_dimension_3 + 3
            }
            AuxiliaryDataType::F32Vec3(val) => {
                val.size_dimension_1 * val.size_dimension_2 * val.size_dimension_3 + 3
            }
            AuxiliaryDataType::U32Vec4(val) => {
                val.size_dimension_1
                    * val.size_dimension_2
                    * val.size_dimension_3
                    * val.size_dimension_4
                    + 4
            }
            AuxiliaryDataType::F32Vec4(val) => {
                val.size_dimension_1
                    * val.size_dimension_2
                    * val.size_dimension_3
                    * val.size_dimension_4
                    + 4
            }
        }
    }
}

impl AuxiliaryDataTypeConsumer {
    pub fn mem_size(&self) -> u64 {
        match self {
            AuxiliaryDataTypeConsumer::Empty => 0,
            AuxiliaryDataTypeConsumer::U32 => mem::size_of::<u32>() as u64,
            AuxiliaryDataTypeConsumer::F32 => mem::size_of::<f32>() as u64,
            AuxiliaryDataTypeConsumer::U32Vec => mem::size_of::<u32>() as u64,
            AuxiliaryDataTypeConsumer::F32Vec => mem::size_of::<f32>() as u64,
            AuxiliaryDataTypeConsumer::U32Vec2 => mem::size_of::<u32>() as u64,
            AuxiliaryDataTypeConsumer::F32Vec2 => mem::size_of::<f32>() as u64,
            AuxiliaryDataTypeConsumer::U32Vec3 => mem::size_of::<u32>() as u64,
            AuxiliaryDataTypeConsumer::F32Vec3 => mem::size_of::<f32>() as u64,
            AuxiliaryDataTypeConsumer::U32Vec4 => mem::size_of::<u32>() as u64,
            AuxiliaryDataTypeConsumer::F32Vec4 => mem::size_of::<f32>() as u64,
        }
    }

    pub fn empty_buffer(&self) -> Vec<u8> {
        match self {
            AuxiliaryDataTypeConsumer::Empty => vec![],
            AuxiliaryDataTypeConsumer::U32 => 0_u32.to_be_bytes().to_vec(), // Default value: 0
            AuxiliaryDataTypeConsumer::F32 => 0.0_f32.to_be_bytes().to_vec(), // Default value: 0.0
            AuxiliaryDataTypeConsumer::U32Vec => {
                let empty: Vec<u32> = vec![0]; // Need to pass at least 1 value
                vec![
                    0_u32.to_be_bytes().to_vec(), // size: 0
                    bytemuck::cast_slice(&empty).to_vec(),
                ]
                .concat()
            }
            AuxiliaryDataTypeConsumer::F32Vec => {
                let empty: Vec<f32> = vec![0.0];
                vec![
                    0_u32.to_be_bytes().to_vec(), // size: 0
                    bytemuck::cast_slice(&empty).to_vec(),
                ]
                .concat()
            }
            AuxiliaryDataTypeConsumer::U32Vec2 => {
                let empty: Vec<u32> = vec![0];
                vec![
                    0_u32.to_be_bytes().to_vec(), // size_0: 0
                    0_u32.to_be_bytes().to_vec(), // size_1: 0
                    bytemuck::cast_slice(&empty).to_vec(),
                ]
                .concat()
            }
            AuxiliaryDataTypeConsumer::F32Vec2 => {
                let empty: Vec<f32> = vec![0.0];
                vec![
                    0_u32.to_be_bytes().to_vec(), // size_0: 0
                    0_u32.to_be_bytes().to_vec(), // size_1: 0
                    bytemuck::cast_slice(&empty).to_vec(),
                ]
                .concat()
            }
            AuxiliaryDataTypeConsumer::U32Vec3 => {
                let empty: Vec<u32> = vec![0];
                vec![
                    0_u32.to_be_bytes().to_vec(), // size_0: 0
                    0_u32.to_be_bytes().to_vec(), // size_1: 0
                    0_u32.to_be_bytes().to_vec(), // size_2: 0
                    bytemuck::cast_slice(&empty).to_vec(),
                ]
                .concat()
            }
            AuxiliaryDataTypeConsumer::F32Vec3 => {
                let empty: Vec<f32> = vec![0.0];
                vec![
                    0_u32.to_be_bytes().to_vec(), // size_0: 0
                    0_u32.to_be_bytes().to_vec(), // size_1: 0
                    0_u32.to_be_bytes().to_vec(), // size_2: 0
                    bytemuck::cast_slice(&empty).to_vec(),
                ]
                .concat()
            }
            AuxiliaryDataTypeConsumer::U32Vec4 => {
                let empty: Vec<u32> = vec![0];
                vec![
                    0_u32.to_be_bytes().to_vec(), // size_0: 0
                    0_u32.to_be_bytes().to_vec(), // size_1: 0
                    0_u32.to_be_bytes().to_vec(), // size_2: 0
                    0_u32.to_be_bytes().to_vec(), // size_3: 0
                    bytemuck::cast_slice(&empty).to_vec(),
                ]
                .concat()
            }
            AuxiliaryDataTypeConsumer::F32Vec4 => {
                let empty: Vec<f32> = vec![0.0];
                vec![
                    0_u32.to_be_bytes().to_vec(), // size_0: 0
                    0_u32.to_be_bytes().to_vec(), // size_1: 0
                    0_u32.to_be_bytes().to_vec(), // size_2: 0
                    0_u32.to_be_bytes().to_vec(), // size_3: 0
                    bytemuck::cast_slice(&empty).to_vec(),
                ]
                .concat()
            }
        }
    }
}

impl Display for AuxiliaryDataTypeConsumer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuxiliaryDataTypeConsumer::Empty => write!(f, "Empty"),
            AuxiliaryDataTypeConsumer::U32 => write!(f, "U32"),
            AuxiliaryDataTypeConsumer::F32 => write!(f, "F32"),
            AuxiliaryDataTypeConsumer::U32Vec => write!(f, "U32Vec"),
            AuxiliaryDataTypeConsumer::F32Vec => write!(f, "F32Vec"),
            AuxiliaryDataTypeConsumer::U32Vec2 => write!(f, "U32Vec2"),
            AuxiliaryDataTypeConsumer::F32Vec2 => write!(f, "F32Vec2"),
            AuxiliaryDataTypeConsumer::U32Vec3 => write!(f, "U32Vec3"),
            AuxiliaryDataTypeConsumer::F32Vec3 => write!(f, "F32Vec3"),
            AuxiliaryDataTypeConsumer::U32Vec4 => write!(f, "U32Vec4"),
            AuxiliaryDataTypeConsumer::F32Vec4 => write!(f, "F32Vec4"),
        }
    }
}

pub fn aux_data_is_compatible(
    data: &AuxiliaryDataType,
    consumer: &AuxiliaryDataTypeConsumer,
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

pub fn aux_data_consumer_type_is_compatible(
    consumer_type_a: &AuxiliaryDataTypeConsumer,
    consumer_type_b: &AuxiliaryDataTypeConsumer,
) -> bool {
    consumer_type_a == consumer_type_b
}

// TODO: From/Into
pub fn aux_data_to_consumer_type(data: &AuxiliaryDataType) -> AuxiliaryDataTypeConsumer {
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
