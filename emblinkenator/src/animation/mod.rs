use serde::{Deserialize, Serialize};

use crate::{
    auxiliary_data::{AuxiliaryDataTypeConsumer, AuxiliaryManifest},
    id::{AnimationId, FixtureId, GroupId, InstallationId},
    world::{context::WorldContext, Coord},
};

use self::factory::AnimationManifest;

pub mod factory;
pub mod manager;

#[derive(Clone, Debug)]
pub struct Animation {
    id: AnimationId,
    manifest: AnimationManifest,
    pub target: AnimationTargetType,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum AnimationTargetType {
    Fixture(FixtureId),
    Installation(InstallationId),
    Group(GroupId),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ShadersConfig {
    shader_folders: Vec<String>,
}

pub trait AnimationTarget {
    fn num_leds(&self, context: &WorldContext) -> u32;
    fn led_positions(&self, context: &WorldContext) -> Vec<Coord>;
}

impl Animation {
    pub fn new(manifest: AnimationManifest, target: AnimationTargetType) -> Animation {
        Animation {
            id: AnimationId::new(),
            manifest,
            target,
        }
    }

    pub fn id(&self) -> AnimationId {
        self.id.clone()
    }

    pub fn get_shader_str(&self) -> String {
        self.manifest.shader.clone()
    }

    pub fn get_target_type(&self) -> AnimationTargetType {
        self.target.clone()
    }

    pub fn get_auxiliaries(&self) -> Option<Vec<AuxiliaryManifest>> {
        self.manifest.auxiliaries.clone()
    }

    pub fn get_auxiliary_data_types(&self) -> Option<Vec<AuxiliaryDataTypeConsumer>> {
        self.manifest
            .auxiliaries
            .clone()
            .map(|auxiliaries| auxiliaries.iter().map(|aux| aux.get_aux_type()).collect())
    }
}

impl Default for ShadersConfig {
    fn default() -> Self {
        Self {
            shader_folders: vec!["shaders".to_string()],
        }
    }
}

impl From<AnimationTargetType> for String {
    fn from(target: AnimationTargetType) -> Self {
        match target {
            AnimationTargetType::Fixture(fixture_id) => fixture_id.unprotect(),
            AnimationTargetType::Installation(installation_id) => installation_id.unprotect(),
            AnimationTargetType::Group(group_id) => group_id.unprotect(),
        }
    }
}
