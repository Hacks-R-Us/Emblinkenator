pub mod manager;

use serde::Deserialize;
use std::convert::TryFrom;
use std::fmt::Display;
use std::mem;
use strum_macros::EnumIter;

#[derive(Debug, Clone)]
pub enum AuxiliaryDataType {
    Empty,
    F32(f32),
    F32Vec(AuxDataF32Vec),
    F32Vec2(AuxDataF32Vec2),
    F32Vec3(AuxDataF32Vec3),
    F32Vec4(AuxDataF32Vec4),
}

#[derive(Debug)]
pub enum AuxDataError {
    IncorrectDimensions,
}

#[derive(Debug, Clone)]
pub struct AuxDataF32Vec {
    data: Vec<f32>,
    size_dimension_1: u32,
}

impl TryFrom<(Vec<f32>, usize)> for AuxDataF32Vec {
    type Error = AuxDataError;

    fn try_from(value: (Vec<f32>, usize)) -> Result<Self, Self::Error> {
        let data = value.0;
        let size_dimension_1 = value.1;

        if size_dimension_1 != data.len() {
            return Err(AuxDataError::IncorrectDimensions);
        }

        Ok(Self {
            data,
            size_dimension_1: size_dimension_1 as u32,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuxDataF32Vec2 {
    data: Vec<f32>,
    size_dimension_1: u32,
    size_dimension_2: u32,
}

impl TryFrom<(Vec<f32>, usize, usize)> for AuxDataF32Vec2 {
    type Error = AuxDataError;

    fn try_from(value: (Vec<f32>, usize, usize)) -> Result<Self, Self::Error> {
        let data = value.0;
        let size_dimension_1 = value.1;
        let size_dimension_2 = value.2;
        let total_size = size_dimension_1 * size_dimension_2;

        if total_size != data.len() {
            return Err(AuxDataError::IncorrectDimensions);
        }

        Ok(Self {
            data,
            size_dimension_1: size_dimension_1 as u32,
            size_dimension_2: size_dimension_2 as u32,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuxDataF32Vec3 {
    data: Vec<f32>,
    size_dimension_1: u32,
    size_dimension_2: u32,
    size_dimension_3: u32,
}

impl TryFrom<(Vec<f32>, usize, usize, usize)> for AuxDataF32Vec3 {
    type Error = AuxDataError;

    fn try_from(value: (Vec<f32>, usize, usize, usize)) -> Result<Self, Self::Error> {
        let data = value.0;
        let size_dimension_1 = value.1;
        let size_dimension_2 = value.2;
        let size_dimension_3 = value.3;
        let total_size = size_dimension_1 * size_dimension_2 * size_dimension_3;

        if total_size != data.len() {
            return Err(AuxDataError::IncorrectDimensions);
        }

        Ok(Self {
            data,
            size_dimension_1: size_dimension_1 as u32,
            size_dimension_2: size_dimension_2 as u32,
            size_dimension_3: size_dimension_3 as u32,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuxDataF32Vec4 {
    data: Vec<f32>,
    size_dimension_1: u32,
    size_dimension_2: u32,
    size_dimension_3: u32,
    size_dimension_4: u32,
}

impl TryFrom<(Vec<f32>, usize, usize, usize, usize)> for AuxDataF32Vec4 {
    type Error = AuxDataError;

    fn try_from(value: (Vec<f32>, usize, usize, usize, usize)) -> Result<Self, Self::Error> {
        let data = value.0;
        let size_dimension_1 = value.1;
        let size_dimension_2 = value.2;
        let size_dimension_3 = value.3;
        let size_dimension_4 = value.4;
        let total_size = size_dimension_1 * size_dimension_2 * size_dimension_3 * size_dimension_4;

        if total_size != data.len() {
            return Err(AuxDataError::IncorrectDimensions);
        }

        Ok(Self {
            data,
            size_dimension_1: size_dimension_1 as u32,
            size_dimension_2: size_dimension_2 as u32,
            size_dimension_3: size_dimension_3 as u32,
            size_dimension_4: size_dimension_4 as u32,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuxiliaryData {
    pub data: AuxiliaryDataType,
    pub size: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum AuxiliaryDataTypeConsumer {
    Empty,
    F32,
    F32Vec,
    F32Vec2,
    F32Vec3,
    F32Vec4,
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
            AuxiliaryDataType::F32(val) => val.to_be_bytes().to_vec(),
            AuxiliaryDataType::F32Vec(val) => vec![
                (val.size_dimension_1).to_be_bytes().to_vec(),
                bytemuck::cast_slice(&val.data).to_vec(),
            ]
            .concat(),
            AuxiliaryDataType::F32Vec2(val) => vec![
                val.size_dimension_1.to_be_bytes().to_vec(),
                val.size_dimension_2.to_be_bytes().to_vec(),
                bytemuck::cast_slice(&val.data).to_vec(),
            ]
            .concat(),
            AuxiliaryDataType::F32Vec3(val) => vec![
                val.size_dimension_1.to_be_bytes().to_vec(),
                val.size_dimension_2.to_be_bytes().to_vec(),
                val.size_dimension_3.to_be_bytes().to_vec(),
                bytemuck::cast_slice(&val.data).to_vec(),
            ]
            .concat(),
            AuxiliaryDataType::F32Vec4(val) => vec![
                val.size_dimension_1.to_be_bytes().to_vec(),
                val.size_dimension_2.to_be_bytes().to_vec(),
                val.size_dimension_3.to_be_bytes().to_vec(),
                val.size_dimension_4.to_be_bytes().to_vec(),
                bytemuck::cast_slice(&val.data).to_vec(),
            ]
            .concat(),
        }
    }

    pub fn get_number_of_values(&self) -> u32 {
        match self {
            AuxiliaryDataType::Empty => 0,
            AuxiliaryDataType::F32(_) => 1,
            AuxiliaryDataType::F32Vec(val) => val.size_dimension_1 + 1,
            AuxiliaryDataType::F32Vec2(val) => val.size_dimension_1 * val.size_dimension_2 + 2,
            AuxiliaryDataType::F32Vec3(val) => {
                val.size_dimension_1 * val.size_dimension_2 * val.size_dimension_3 + 3
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
            AuxiliaryDataTypeConsumer::F32 => mem::size_of::<f32>() as u64,
            AuxiliaryDataTypeConsumer::F32Vec => mem::size_of::<f32>() as u64,
            AuxiliaryDataTypeConsumer::F32Vec2 => mem::size_of::<f32>() as u64,
            AuxiliaryDataTypeConsumer::F32Vec3 => mem::size_of::<f32>() as u64,
            AuxiliaryDataTypeConsumer::F32Vec4 => mem::size_of::<f32>() as u64,
        }
    }

    pub fn empty_buffer(&self) -> Vec<u8> {
        match self {
            AuxiliaryDataTypeConsumer::Empty => vec![],
            AuxiliaryDataTypeConsumer::F32 => 0.0_f32.to_be_bytes().to_vec(), // Default value: 0.0
            AuxiliaryDataTypeConsumer::F32Vec => {
                let empty: Vec<f32> = vec![0.0];
                vec![
                    0_u32.to_be_bytes().to_vec(), // size: 0
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
            AuxiliaryDataTypeConsumer::F32 => write!(f, "F32"),
            AuxiliaryDataTypeConsumer::F32Vec => write!(f, "F32Vec"),
            AuxiliaryDataTypeConsumer::F32Vec2 => write!(f, "F32Vec2"),
            AuxiliaryDataTypeConsumer::F32Vec3 => write!(f, "F32Vec3"),
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
        AuxiliaryDataTypeConsumer::F32 => matches!(data, AuxiliaryDataType::F32(_)),
        AuxiliaryDataTypeConsumer::F32Vec => matches!(data, AuxiliaryDataType::F32Vec(_)),
        AuxiliaryDataTypeConsumer::F32Vec2 => matches!(data, AuxiliaryDataType::F32Vec2(_)),
        AuxiliaryDataTypeConsumer::F32Vec3 => matches!(data, AuxiliaryDataType::F32Vec3(_)),
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
        AuxiliaryDataType::F32(_) => AuxiliaryDataTypeConsumer::F32,
        AuxiliaryDataType::F32Vec(_) => AuxiliaryDataTypeConsumer::F32Vec,
        AuxiliaryDataType::F32Vec2(_) => AuxiliaryDataTypeConsumer::F32Vec2,
        AuxiliaryDataType::F32Vec3(_) => AuxiliaryDataTypeConsumer::F32Vec3,
        AuxiliaryDataType::F32Vec4(_) => AuxiliaryDataTypeConsumer::F32Vec4,
    }
}
