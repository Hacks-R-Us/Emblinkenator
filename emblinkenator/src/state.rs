use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    animation::manager::AnimationManager, pipeline::PipelineContext, world::context::WorldContext,
};

pub struct EmblinkenatorState {
    animation_manager: Arc<RwLock<AnimationManager>>,
    world_context: Arc<RwLock<WorldContext>>,
    pipeline_context_buffer: crossbeam::channel::Sender<PipelineContext>,
}

pub trait ThreadedObject: Sync + Send {
    // Do not loop inside run!
    fn run(&mut self);
}

impl EmblinkenatorState {
    pub fn new(
        animation_manager: Arc<RwLock<AnimationManager>>,
        world_context: Arc<RwLock<WorldContext>>,
        pipeline_context_buffer: crossbeam::channel::Sender<PipelineContext>,
    ) -> EmblinkenatorState {
        EmblinkenatorState {
            animation_manager,
            world_context,
            pipeline_context_buffer,
        }
    }
}

impl ThreadedObject for EmblinkenatorState {
    fn run(&mut self) {
        if self.pipeline_context_buffer.is_full() {
            return;
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

        self.pipeline_context_buffer.send(pipeline_context).unwrap();
    }
}
