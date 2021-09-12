use crate::{
    id::{AnimationId, FixtureId, GroupId, InstallationId},
    world::{context::WorldContext, Coord},
};

use self::factory::AnimationManifest;

pub mod factory;
pub mod manager;

#[derive(Clone)]
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

    pub fn get_target(&self, context: &WorldContext) -> Option<Box<dyn AnimationTarget>> {
        match &self.target {
            AnimationTargetType::Fixture(id) => {
                let fixture = context.get_fixture(id);

                if let Some(fixture) = fixture {
                    return Some(Box::new(fixture));
                }

                None
            }
            AnimationTargetType::Installation(id) => {
                let installation = context.get_installation(id);

                if let Some(installation) = installation {
                    return Some(Box::new(installation));
                }

                None
            }
            AnimationTargetType::Group(id) => {
                let group = context.get_group(id);

                if let Some(group) = group {
                    return Some(Box::new(group));
                }

                None
            }
        }
    }
}
