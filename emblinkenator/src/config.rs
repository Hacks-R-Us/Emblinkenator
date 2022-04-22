use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    animation::ShadersConfig,
    auxiliary_data::{manager::AuxiliaryConfigParams, AuxiliaryDataTypeConsumer},
    devices::manager::DeviceConfigType,
    id::AuxiliaryId,
    world::Coord,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct EmblinkenatorConfig {
    pub frame_buffer_size: u32,
    pub frame_numerator: u32,
    pub frame_denominator: u32,
    pub leds_per_compute_group: u32,
    pub shaders: ShadersConfig,
}

// Temporary solution for adding some devices before a UI comes into existence
#[derive(Deserialize)]
pub struct StartupConfig {
    pub fixtures: Vec<StartupFixture>,
    pub animations: Vec<StartupAnimations>,
    pub auxiliaries: Vec<StartupAuxiliaries>,
    pub devices: Vec<StartupDevice>,
    pub fixtures_to_device: HashMap<String, String>,
    // AnimationId -> [AuxiliaryId]
    pub animation_auxiliary_sources: HashMap<String, Vec<String>>,
}

#[derive(Deserialize)]
pub struct StartupFixture {
    pub id: String,
    pub num_leds: u32,
    pub led_positions: Option<Vec<Coord>>,
}

#[derive(Deserialize)]
pub struct StartupAnimations {
    pub id: String,
    pub shader_id: String,
    pub target_id: StartupAnimationTargetType,
}

#[derive(Deserialize, Clone)]
#[serde(tag = "type")]
pub enum StartupAuxiliaries {
    F32 {
        id: String,
        name: String,
        initial_value: Option<f32>,
        max_value: Option<f32>,
        min_value: Option<f32>,
    },
    F32Vec {
        id: String,
        name: String,
    },
    F32Vec2 {
        id: String,
        name: String,
    },
    F32Vec3 {
        id: String,
        name: String,
    },
    F32Vec4 {
        id: String,
        name: String,
    },
}

impl StartupAuxiliaries {
    pub fn get_name(&self) -> String {
        match self {
            StartupAuxiliaries::F32 {
                id: _,
                name,
                initial_value: _,
                max_value: _,
                min_value: _,
            } => name.clone(),
            StartupAuxiliaries::F32Vec { id: _, name } => name.clone(),
            StartupAuxiliaries::F32Vec2 { id: _, name } => name.clone(),
            StartupAuxiliaries::F32Vec3 { id: _, name } => name.clone(),
            StartupAuxiliaries::F32Vec4 { id: _, name } => name.clone(),
        }
    }
}

impl From<StartupAuxiliaries> for AuxiliaryId {
    fn from(aux: StartupAuxiliaries) -> Self {
        match aux {
            StartupAuxiliaries::F32 {
                id,
                name: _,
                initial_value: _,
                max_value: _,
                min_value: _,
            } => AuxiliaryId::new_from(id),
            StartupAuxiliaries::F32Vec { id, name: _ } => AuxiliaryId::new_from(id),
            StartupAuxiliaries::F32Vec2 { id, name: _ } => AuxiliaryId::new_from(id),
            StartupAuxiliaries::F32Vec3 { id, name: _ } => AuxiliaryId::new_from(id),
            StartupAuxiliaries::F32Vec4 { id, name: _ } => AuxiliaryId::new_from(id),
        }
    }
}

impl From<StartupAuxiliaries> for AuxiliaryDataTypeConsumer {
    fn from(aux: StartupAuxiliaries) -> Self {
        match aux {
            StartupAuxiliaries::F32 {
                id: _,
                name: _,
                initial_value: _,
                max_value: _,
                min_value: _,
            } => AuxiliaryDataTypeConsumer::F32,
            StartupAuxiliaries::F32Vec { id: _, name: _ } => AuxiliaryDataTypeConsumer::F32Vec,
            StartupAuxiliaries::F32Vec2 { id: _, name: _ } => AuxiliaryDataTypeConsumer::F32Vec2,
            StartupAuxiliaries::F32Vec3 { id: _, name: _ } => AuxiliaryDataTypeConsumer::F32Vec3,
            StartupAuxiliaries::F32Vec4 { id: _, name: _ } => AuxiliaryDataTypeConsumer::F32Vec4,
        }
    }
}

impl From<StartupAuxiliaries> for AuxiliaryConfigParams {
    fn from(aux: StartupAuxiliaries) -> Self {
        match aux {
            StartupAuxiliaries::F32 {
                id: _,
                name: _,
                initial_value,
                max_value,
                min_value,
            } => AuxiliaryConfigParams::F32 {
                initial_value: initial_value.unwrap_or(0.0),
                max_value: max_value.unwrap_or(1.0).clamp(0.0, 1.0),
                min_value: min_value.unwrap_or(0.0).clamp(0.0, 1.0),
            },
            StartupAuxiliaries::F32Vec { id: _, name: _ } => AuxiliaryConfigParams::F32Vec,
            StartupAuxiliaries::F32Vec2 { id: _, name: _ } => AuxiliaryConfigParams::F32Vec2,
            StartupAuxiliaries::F32Vec3 { id: _, name: _ } => AuxiliaryConfigParams::F32Vec3,
            StartupAuxiliaries::F32Vec4 { id: _, name: _ } => AuxiliaryConfigParams::F32Vec4,
        }
    }
}

#[derive(Deserialize)]
pub struct StartupDevice {
    pub id: String,
    pub config: DeviceConfigType,
}

#[derive(Deserialize)]
pub enum StartupAnimationTargetType {
    Fixture(String),
    Installation(String),
    Group(String),
}

impl EmblinkenatorConfig {
    pub fn frame_buffer_size(&self) -> u32 {
        self.frame_buffer_size
    }

    pub fn frame_numerator(&self) -> u32 {
        self.frame_numerator
    }

    pub fn frame_denominator(&self) -> u32 {
        self.frame_denominator
    }

    pub fn leds_per_compute_group(&self) -> u32 {
        self.leds_per_compute_group
    }
}

impl Default for EmblinkenatorConfig {
    fn default() -> Self {
        Self {
            frame_buffer_size: 10,
            frame_numerator: 1000,
            frame_denominator: 25,
            leds_per_compute_group: 64,
            shaders: ShadersConfig::default(),
        }
    }
}
