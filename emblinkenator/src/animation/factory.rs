use std::collections::HashMap;

const BUILTIN_ANIMATIONS: &[(&str, &str)] = &[
    ("test", include_str!("../shaders/test.wgsl")),
    ("colourfade", include_str!("../shaders/colourfade.wgsl")),
];

#[derive(Clone)]
pub struct AnimationManifest {
    pub shader: String,
}

impl AnimationManifest {
    fn new(shader: String) -> AnimationManifest {
        AnimationManifest { shader }
    }
}

pub struct AnimationRegistry {
    animations: HashMap<String, AnimationManifest>,
}

impl AnimationRegistry {
    fn new() -> AnimationRegistry {
        AnimationRegistry {
            animations: HashMap::new(),
        }
    }

    fn register(
        &mut self,
        id: String,
        shader: String,
    ) -> Result<(), AnimationRegistryRegisterError> {
        if self.animations.contains_key(&id) {
            return Err(AnimationRegistryRegisterError::AnimationExistsWithId);
        }

        let manifest = AnimationManifest::new(shader);
        self.animations.insert(id, manifest);

        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<AnimationManifest> {
        self.animations.get(id).cloned()
    }
}

#[derive(Debug, PartialEq)]
enum AnimationRegistryError {
    AnimationRegistryErrorRegister(AnimationRegistryRegisterError),
}

#[derive(Debug, PartialEq)]
enum AnimationRegistryRegisterError {
    AnimationExistsWithId,
}

pub fn get_animation_registry() -> AnimationRegistry {
    let mut registry = AnimationRegistry::new();

    register_builtin_animations(&mut registry);

    registry
}

fn register_builtin_animations(registry: &mut AnimationRegistry) {
    for animation in BUILTIN_ANIMATIONS.iter() {
        registry
            .register(animation.0.to_string(), animation.1.to_string())
            .unwrap();
    }
}
