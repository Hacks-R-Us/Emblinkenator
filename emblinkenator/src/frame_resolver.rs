use std::{collections::HashMap, sync::Arc};

use log::debug;
use parking_lot::RwLock;
use tokio::sync::broadcast::{self, Receiver};

use crate::{
    animation::{manager::AnimationManager, AnimationTargetType},
    id::{AnimationId, FixtureId},
    led::LED,
    state::ThreadedObject,
    world::context::WorldContext,
};

const DATA_EVENT_CHANNEL_CAPACITY: usize = 60;

pub struct FrameResolver {
    input_data_buffer: crossbeam::channel::Receiver<FrameResolverInput>,
    animation_manager: Arc<RwLock<AnimationManager>>,
    world_context: Arc<RwLock<WorldContext>>,
    event_emitters: FrameResolverEventEmitters,
}

struct FrameResolverEventEmitters {
    data: broadcast::Sender<FrameResolverDataEvent>,
}

#[derive(Debug, Clone)]
pub struct FrameResolverDataEvent {
    pub target: FixtureId,
    pub data: Vec<LED>,
}

pub struct FrameResolverInput {
    pub states: HashMap<String, Vec<LED>>,
}

struct FrameIntermediate {
    data: Vec<LED>,
    priority: u32,
}

impl FrameResolver {
    pub fn new(
        animation_manager: Arc<RwLock<AnimationManager>>,
        world_context: Arc<RwLock<WorldContext>>,
        input_data_buffer: crossbeam::channel::Receiver<FrameResolverInput>,
    ) -> FrameResolver {
        FrameResolver {
            input_data_buffer,
            animation_manager,
            world_context,
            event_emitters: FrameResolverEventEmitters::new(),
        }
    }

    pub fn subscribe_to_resolved_frames(&self) -> Receiver<FrameResolverDataEvent> {
        self.event_emitters.data.subscribe()
    }
}

impl ThreadedObject for FrameResolver {
    fn run(&mut self) {
        let mut intermediate_data: HashMap<AnimationTargetType, FrameIntermediate> = HashMap::new(); // TODO: Map<LayerId, Vec<FrameIntermediate>>
        let mut compute_outputs: Vec<FrameResolverInput> = vec![];

        debug!("Recv data");

        let message = self.input_data_buffer.recv();
        match message {
            Ok(msg) => compute_outputs.push(msg),
            Err(_) => {
                panic!("Pipeline message queue closed early");
            }
        }

        {
            let animation_manager = self.animation_manager.read();
            for (index, compute_output) in compute_outputs.iter().enumerate() {
                for (animation_id, data) in compute_output.states.iter() {
                    let animation_id = AnimationId::new_from(animation_id.clone());
                    let animation = animation_manager.get_animation(&animation_id);

                    if animation.is_none() {
                        debug!(
                            "Could not find animation {} in frame resolver",
                            animation_id.unprotect()
                        );
                        break;
                    }

                    let target = animation.unwrap().get_target_type();

                    intermediate_data.insert(
                        target,
                        FrameIntermediate {
                            data: data.clone(),
                            priority: index as u32, // TODO: Animation priorities
                        },
                    );
                }
            }
        }

        // TODO: Merge/combine values on the same target by priority / merge rules
        for (target, data) in intermediate_data {
            if self.event_emitters.data.receiver_count() > 0 {
                let mut chunks: Vec<(FixtureId, u32)> = vec![];
                match target {
                    AnimationTargetType::Fixture(fixture_id) => {
                        if let Some(fixture) = self.world_context.read().get_fixture(&fixture_id) {
                            chunks = vec![(fixture.id().clone(), fixture.led_count())]
                        }
                    }
                    AnimationTargetType::Installation(installation_id) => {
                        if let Some(installation) =
                            self.world_context.read().get_installation(&installation_id)
                        {
                            chunks = installation.get_fixture_chunks(&self.world_context.read());
                        }
                    }
                    AnimationTargetType::Group(group_id) => {
                        if let Some(group) = self.world_context.read().get_group(&group_id) {
                            chunks = group.get_fixture_chunks(&self.world_context.read());
                        }
                    }
                };

                let current_position = 0;

                for chunk in chunks {
                    let mut fixture_data = vec![LED::default(); chunk.1 as usize];
                    if (current_position + chunk.1 as usize) < data.data.len() {
                        let data = data
                            .data
                            .get(current_position..current_position + chunk.1 as usize);
                        if let Some(data) = data {
                            fixture_data = data.to_vec();
                        }
                    } else if current_position < data.data.len() {
                        let data = data.data.get(current_position..);
                        if let Some(data) = data {
                            fixture_data = data.to_vec();
                        }
                    }
                    self.event_emitters
                        .data
                        .send(FrameResolverDataEvent::new(&chunk.0, &fixture_data))
                        .ok();
                }
            }
        }
    }
}

impl FrameResolverEventEmitters {
    fn new() -> Self {
        FrameResolverEventEmitters {
            data: broadcast::channel(DATA_EVENT_CHANNEL_CAPACITY).0,
        }
    }
}

impl FrameResolverDataEvent {
    fn new(target: &FixtureId, data: &[LED]) -> Self {
        FrameResolverDataEvent {
            target: target.clone(),
            data: data.to_vec(),
        }
    }
}
