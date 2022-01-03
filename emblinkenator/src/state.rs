use std::sync::Arc;

use crossbeam::channel::Sender;
use parking_lot::RwLock;

use crate::{
    animation::manager::AnimationManager, auxiliary_data::AuxiliaryDataManager,
    frame::FrameTimeKeeper, pipeline::PipelineContext, world::context::WorldContext,
};

pub struct EmblinkenatorState {
    animation_manager: Arc<RwLock<AnimationManager>>,
    auxiliary_data_manager: Arc<RwLock<AuxiliaryDataManager>>,
    frame_time_keeper: Arc<RwLock<FrameTimeKeeper>>,
    world_context: Arc<RwLock<WorldContext>>,
    pipeline_context_subscribers: Vec<crossbeam::channel::Sender<PipelineContext>>,
}

pub trait ThreadedObject: Sync + Send {
    // Do not loop inside run!
    fn run(&mut self);
}

impl EmblinkenatorState {
    pub fn new(
        animation_manager: Arc<RwLock<AnimationManager>>,
        auxiliary_data_manager: Arc<RwLock<AuxiliaryDataManager>>,
        frame_time_keeper: Arc<RwLock<FrameTimeKeeper>>,
        world_context: Arc<RwLock<WorldContext>>,
    ) -> EmblinkenatorState {
        EmblinkenatorState {
            animation_manager,
            auxiliary_data_manager,
            frame_time_keeper,
            world_context,
            pipeline_context_subscribers: vec![],
        }
    }

    pub fn send_pipeline_context_to(&mut self, pipeline_context_buffer: Sender<PipelineContext>) {
        self.pipeline_context_subscribers
            .push(pipeline_context_buffer);
    }
}

impl ThreadedObject for EmblinkenatorState {
    fn run(&mut self) {
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
                let animations = self.animation_manager.read().get_animation_states();
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
    }
}
