use std::collections::HashMap;
use std::sync::Mutex;

use crate::id::AnimationId;

use super::{Animation, AnimationTargetType, ShadersConfig, factory::{get_animation_registry, AnimationRegistry}};

pub struct AnimationManager {
    registry: AnimationRegistry,
    animations: Mutex<HashMap<AnimationId, Animation>>,
}

#[derive(Debug)]
pub enum AnimationManagerError {
    AnimationIsNotRegistered,
}

pub trait RecvAnimationManagerState: Send + Sync {
    fn recv(&self, state: HashMap<AnimationId, Animation>);
}

impl AnimationManager {
    pub fn new(config: &ShadersConfig) -> AnimationManager {
        let animation_registry = get_animation_registry(config);

        AnimationManager {
            registry: animation_registry,
            animations: Mutex::new(HashMap::new()),
        }
    }

    pub fn create_animation(
        &self,
        shader_id: String,
        target: AnimationTargetType,
    ) -> Result<AnimationId, AnimationManagerError> {
        let manifest = self.registry.get(&shader_id);

        if manifest.is_none() {
            return Err(AnimationManagerError::AnimationIsNotRegistered);
        }

        let manifest = manifest.unwrap();

        let animation = Animation::new(manifest, target);

        let id = animation.id();

        self.animations
            .lock()
            .unwrap()
            .insert(id.clone(), animation);

        Ok(id)
    }

    pub fn destroy_animation(&self, _id: &AnimationId) {
        // TODO
    }

    pub fn get_animation(&self, id: &AnimationId) -> Option<Animation> {
        self.animations.lock().unwrap().get(id).cloned()
    }

    pub fn get_animation_states(&self) -> HashMap<AnimationId, Animation> {
        self.animations.lock().unwrap().clone()
    }
}
