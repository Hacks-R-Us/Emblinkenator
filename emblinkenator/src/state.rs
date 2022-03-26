use std::sync::Arc;

use crossbeam::channel::Sender;

use parking_lot::RwLock;

use crate::{
    animation::manager::AnimationManager,
    auxiliary_data::AuxiliaryDataManager,
    devices::manager::{DeviceManager, DeviceManagerEvent, ThreadedDeviceType},
    frame::FrameTimeKeeper,
    frame_resolver::FrameResolver,
    id::DeviceId,
    pipeline::PipelineContext,
    world::context::WorldContext,
};

pub struct EmblinkenatorState {
    animation_manager: Arc<RwLock<AnimationManager>>,
    auxiliary_data_manager: Arc<RwLock<AuxiliaryDataManager>>,
    device_manager: Arc<RwLock<DeviceManager>>,
    frame_time_keeper: Arc<RwLock<FrameTimeKeeper>>,
    frame_resolver: Arc<RwLock<FrameResolver>>,
    world_context: Arc<RwLock<WorldContext>>,
    pipeline_context_subscribers: Vec<crossbeam::channel::Sender<PipelineContext>>,
    wants_device_state: Vec<Arc<RwLock<dyn WantsDeviceState>>>,
    device_manager_events: crossbeam::channel::Receiver<DeviceManagerEvent>,
}

pub trait ThreadedObject: Sync + Send {
    /// Do not loop inside tick!
    fn tick(&mut self);
}

pub trait WantsDeviceState: Sync + Send {
    fn on_device_added(&mut self, state: &EmblinkenatorState, device_id: DeviceId);
}

impl EmblinkenatorState {
    pub fn new(
        animation_manager: Arc<RwLock<AnimationManager>>,
        auxiliary_data_manager: Arc<RwLock<AuxiliaryDataManager>>,
        device_manager: Arc<RwLock<DeviceManager>>,
        frame_time_keeper: Arc<RwLock<FrameTimeKeeper>>,
        frame_resolver: Arc<RwLock<FrameResolver>>,
        world_context: Arc<RwLock<WorldContext>>,
    ) -> EmblinkenatorState {
        let device_manager_events = device_manager.write().subscribe_to_events();
        EmblinkenatorState {
            animation_manager: Arc::clone(&animation_manager),
            auxiliary_data_manager: Arc::clone(&auxiliary_data_manager),
            device_manager,
            frame_time_keeper: Arc::clone(&frame_time_keeper),
            frame_resolver: Arc::clone(&frame_resolver),
            world_context,
            pipeline_context_subscribers: vec![],
            wants_device_state: vec![auxiliary_data_manager, frame_time_keeper, frame_resolver],
            device_manager_events,
        }
    }

    pub fn send_pipeline_context_to(&mut self, pipeline_context_buffer: Sender<PipelineContext>) {
        self.pipeline_context_subscribers
            .push(pipeline_context_buffer);
    }

    pub fn get_device(&self, id: &DeviceId) -> Option<Arc<RwLock<ThreadedDeviceType>>> {
        self.device_manager.write().get_device(id)
    }
}

impl ThreadedObject for EmblinkenatorState {
    fn tick(&mut self) {
        for pipeline_context_buffer in &self.pipeline_context_subscribers {
            if pipeline_context_buffer.is_full() {
                continue;
            }

            let mut pipeline_context = PipelineContext::new();
            {
                let world_context = self.world_context.read().get_world_context_state();
                pipeline_context.led_positions = world_context.led_positions;
                pipeline_context.num_leds = world_context.num_leds;
            }

            {
                let mut animations = self.animation_manager.read().get_animation_states();
                // We must only report animations that are valid for creation
                // i.e. Ones where the number of LEDs of the target are known
                animations.retain(|_, animation| {
                    let target: String = animation.get_target_type().into();
                    pipeline_context.num_leds.get(&target).is_some()
                });
                pipeline_context.animations = animations;
            }

            {
                let auxiliary_data = self.auxiliary_data_manager.read().get_auxiliary_data();
                pipeline_context.auxiliary_data = auxiliary_data;
            }

            {
                let animation_auxiliary_data = self
                    .auxiliary_data_manager
                    .read()
                    .get_animation_auxiliary_ids();
                pipeline_context.animation_auxiliary_data = animation_auxiliary_data;
            }

            pipeline_context_buffer.send(pipeline_context).unwrap();
        }

        while let Ok(device_manager_message) = self.device_manager_events.try_recv() {
            for device in &self.wants_device_state {
                match &device_manager_message {
                    DeviceManagerEvent::DeviceAdded(device_id) => {
                        device.write().on_device_added(self, device_id.clone())
                    }
                    DeviceManagerEvent::DeviceRemoved(_) => todo!(),
                }
            }
        }
    }
}
