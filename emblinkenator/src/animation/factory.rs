use glob::glob;
use log::{error, info};
use serde::Deserialize;
use std::{collections::HashMap, fs};

use crate::auxiliary_data::AuxiliaryManifest;

use super::ShadersConfig;

#[derive(Clone, Debug)]
pub struct AnimationManifest {
    pub shader: String,
    pub auxiliaries: Option<Vec<AuxiliaryManifest>>,
}

#[derive(Debug, Deserialize)]
pub struct ShaderManifest {
    pub id: String,
    pub shader: String,
    pub auxiliaries: Option<Vec<AuxiliaryManifest>>,
}

impl AnimationManifest {
    fn new(shader: String, auxiliaries: Option<Vec<AuxiliaryManifest>>) -> AnimationManifest {
        AnimationManifest {
            shader,
            auxiliaries,
        }
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
        auxiliaries: Option<Vec<AuxiliaryManifest>>,
    ) -> Result<(), AnimationRegistryRegisterError> {
        if self.animations.contains_key(&id) {
            return Err(AnimationRegistryRegisterError::AnimationExistsWithId);
        }

        let manifest = AnimationManifest::new(shader, auxiliaries);
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

pub fn get_animation_registry(config: &ShadersConfig) -> AnimationRegistry {
    let mut registry = AnimationRegistry::new();

    register_animations_from_config(config, &mut registry);

    registry
}

fn register_animations_from_config(config: &ShadersConfig, registry: &mut AnimationRegistry) {
    for folder in config.shader_folders.iter() {
        info!("Loading shaders from folder {}", folder);
        for entry in glob(&format!("{}/**/*.json", folder)).expect("Failed to read glob pattern") {
            if entry.is_err() {
                continue;
            }
            let shader_manifest_path = entry.unwrap();
            let shader_file_contents = fs::read_to_string(shader_manifest_path.clone());
            if shader_file_contents.is_err() {
                error!(
                    "Unable to read shader maifest file {}",
                    shader_manifest_path.display()
                );
                continue;
            }
            let shader_file_contents = shader_file_contents.unwrap();

            let shader_manifest: Result<ShaderManifest, _> =
                serde_json::from_str(&shader_file_contents);
            if shader_manifest.is_err() {
                error!(
                    "Cannot read config data for shader {} ({})",
                    shader_manifest_path.display(),
                    shader_manifest.err().unwrap()
                );
                continue;
            }
            let shader_manifest = shader_manifest.unwrap();

            let mut shader_path = shader_manifest_path.clone();
            shader_path.pop();
            shader_path.push(shader_manifest.shader);

            let shader = fs::read_to_string(shader_path.clone());
            if shader.is_err() {
                error!("Unable to read shader {}", shader_path.display());
                continue;
            }
            let shader = shader.unwrap();

            info!("Registering shader {}", shader_manifest.id);
            if let Err(err) = registry.register(
                shader_manifest.id.clone(),
                shader,
                shader_manifest.auxiliaries,
            ) {
                error!("Error registering shader {}, {:?}", shader_manifest.id, err);
            }
        }
    }
}
