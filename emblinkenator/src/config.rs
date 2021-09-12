use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{devices::manager::DeviceConfigType, world::Coord};

#[derive(Debug, Deserialize, Serialize)]
pub struct EmblinkenatorConfig {
    frame_buffer_size: u32,
    frame_rate: u32,
    leds_per_compute_group: u32
}

impl EmblinkenatorConfig {
    pub fn new (frame_buffer_size: u32, frame_rate: u32, leds_per_compute_group: u32) -> Self {
        EmblinkenatorConfig {
            frame_buffer_size,
            frame_rate,
            leds_per_compute_group
        }
    }

    pub fn frame_buffer_size (&self) -> u32 {
        self.frame_buffer_size
    }

    pub fn frame_rate (&self) -> u32 {
        self.frame_rate
    }

    pub fn leds_per_compute_group (&self) -> u32 {
        self.leds_per_compute_group
    }
}

// Temporary solution for adding some devices before a UI comes into existence
#[derive(Deserialize)]
pub struct StartupConfig {
    pub fixtures: Vec<StartupFixtures>,
    pub animations: Vec<StartupAnimations>,
    pub devices: Vec<StartupDevices>,
    pub fixtures_to_device: HashMap<String, String>
}

#[derive(Deserialize)]
pub struct StartupFixtures {
    pub id: String,
    pub num_leds: u32,
    pub led_positions: Option<Vec<Coord>>
}

#[derive(Deserialize)]
pub struct StartupAnimations {
    pub shader_id: String,
    pub target_id: StartupAnimationTargetType
}

#[derive(Deserialize)]
pub struct StartupDevices {
    pub id: String,
    pub config: DeviceConfigType
}

#[derive(Deserialize)]
pub enum StartupAnimationTargetType {
    Fixture(String),
    Installation(String),
    Group(String)
}
