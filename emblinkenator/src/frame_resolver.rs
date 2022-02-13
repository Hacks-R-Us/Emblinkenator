use std::{collections::HashMap, sync::Arc};

use log::{debug, warn};
use parking_lot::RwLock;
use tokio::sync::broadcast::{Receiver, error::TryRecvError, Sender};

use crate::{animation::{manager::AnimationManager, AnimationTargetType}, event_loop::PipelineFrameOutput, id::{AnimationId, FixtureId, DeviceId}, led::LED, state::{ThreadedObject, WantsDeviceState}, world::context::WorldContext};

pub type LEDFrame = Vec<LED>;

pub struct FrameResolver {
    input_data_buffer: Receiver<PipelineFrameOutput>,
    animation_manager: Arc<RwLock<AnimationManager>>,
    world_context: Arc<RwLock<WorldContext>>,
    fixture_to_device: HashMap<FixtureId, DeviceId>,
    device_buffers: HashMap<DeviceId, Sender<LEDFrame>>
}

struct FrameIntermediate {
    data: Vec<LED>,
    priority: u32,
}

impl FrameResolver {
    pub fn new(
        animation_manager: Arc<RwLock<AnimationManager>>,
        world_context: Arc<RwLock<WorldContext>>,
        input_data_buffer: Receiver<PipelineFrameOutput>,
    ) -> FrameResolver {
        FrameResolver {
            input_data_buffer,
            animation_manager,
            world_context,
            fixture_to_device: HashMap::new(),
            device_buffers: HashMap::new(),
        }
    }

    pub fn set_fixture_to_device(&mut self, fixture_id: FixtureId, device_id: DeviceId) {
        self.fixture_to_device.insert(fixture_id, device_id);
    }

    pub fn set_device_buffer(&mut self, device_id: DeviceId, sender: Sender<LEDFrame>) {
        self.device_buffers.insert(device_id, sender);
    }
}

impl ThreadedObject for FrameResolver {
    fn tick(&mut self) {
        let mut intermediate_data: HashMap<AnimationTargetType, FrameIntermediate> = HashMap::new(); // TODO: Map<LayerId, Vec<FrameIntermediate>>
        let mut compute_outputs: Vec<PipelineFrameOutput> = vec![];

        let message = self.input_data_buffer.try_recv();
        match message {
            Ok(msg) => compute_outputs.push(msg),
            Err(err) => {
                match err {
                    TryRecvError::Lagged(frames) => warn!("Frame resolver lagged by {} frames", frames),
                    TryRecvError::Closed => panic!("Pipeline message queue closed early"),
                    TryRecvError::Empty => {}
                }
            }
        }

        if compute_outputs.len() == 0 {
            return
        }

        debug!("Frame Resolver received {} compute outputs", compute_outputs.len());

        {
            let animation_manager = self.animation_manager.read();
            for (index, compute_output) in compute_outputs.iter().enumerate() {
                debug!("Compute output {} has {} states", index, compute_output.states.len());
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

        debug!("Frame Resolver has {} intermediate values", intermediate_data.keys().len());

        // TODO: Merge/combine values on the same target by priority / merge rules
        for (target, data) in intermediate_data {
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

            for (fixture_id, chunk_size) in chunks {
                let mut fixture_data = vec![LED::default(); chunk_size as usize];
                if (current_position + chunk_size as usize) < data.data.len() {
                    let data = data
                        .data
                        .get(current_position..current_position + chunk_size as usize);
                    if let Some(data) = data {
                        fixture_data = data.to_vec();
                    }
                } else if current_position < data.data.len() {
                    let data = data.data.get(current_position..);
                    if let Some(data) = data {
                        fixture_data = data.to_vec();
                    }
                }

                let device_id = self.fixture_to_device.get(&fixture_id);
                if device_id.is_none() {
                    warn!("Fixture {} is not mapped to a valid device", fixture_id.unprotect());
                    continue;
                }

                let device_id = device_id.unwrap();
                if let Some(sender) = self.device_buffers.get(device_id) {
                    debug!("Sending frame to {}", device_id);
                    sender.send(fixture_data).ok();
                } else {
                    warn!("No data buffer exists for {}", device_id.unprotect())
                }
            }
        }
    }
}

impl WantsDeviceState for FrameResolver {
    fn on_device_added(&mut self, state: &crate::state::EmblinkenatorState, device_id: DeviceId) {
        if let Some(device) = state.get_device(&device_id) {
            match &mut *device.write() {
                crate::devices::manager::ThreadedDeviceType::LEDDataOutput(led_output_device) => {
                    let (sender, receiver) = tokio::sync::broadcast::channel(1);
                    led_output_device.receive_data_from(receiver);
                    self.set_device_buffer(device_id, sender);
                },
                crate::devices::manager::ThreadedDeviceType::AuxiliaryData(_) => {}, // Nothing to do
            }
        }
    }
}
