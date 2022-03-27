use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{animation::ShadersConfig, devices::manager::DeviceConfigType, world::Coord};

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

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum StartupAuxiliaries {
    F32 { id: String, initial_value: f32 },
    F32Vec { id: String },
    F32Vec2 { id: String },
    F32Vec3 { id: String },
    F32Vec4 { id: String },
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
    pub fn new(
        frame_buffer_size: u32,
        frame_numerator: u32,
        frame_denominator: u32,
        leds_per_compute_group: u32,
        shaders: ShadersConfig,
    ) -> Self {
        EmblinkenatorConfig {
            frame_buffer_size,
            frame_numerator,
            frame_denominator,
            leds_per_compute_group,
            shaders,
        }
    }

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
