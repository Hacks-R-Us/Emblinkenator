use std::collections::HashMap;

use crossbeam::channel::Receiver;
use log::debug;

use crate::{
    frame::FrameData,
    led::LED,
    pipeline::{ComputeOutput, EmblinkenatorPipeline, PipelineContext},
};

pub struct GPUEventLoop {
    state: GPUEventLoopState,
    command_queue: Vec<GPUEventLoopQueue>,
    pipeline: EmblinkenatorPipeline,
    frame_data_buffer: Receiver<FrameData>,
    pipeline_context_buffer: crossbeam::channel::Receiver<PipelineContext>,
    frame_output_buffer: tokio::sync::broadcast::Sender<PipelineFrameOutput>,
    frame_state: Option<EventLoopFrameState>,
}

enum GPUEventLoopState {
    Paused,
    BeforeFrame,
    Compute,
    WaitForGPUIdle,
    ReadDataFromGPU,
    OutputData,
    FrameEnd,
}

#[derive(Debug, PartialEq)]
pub enum GPUEventLoopQueue {
    Exit,
}

#[derive(Clone)]
struct EventLoopFrameState {
    pub last_frame_state: Option<ComputeOutput>,
}

#[derive(Debug, Clone)]
pub struct PipelineFrameOutput {
    pub states: HashMap<String, Vec<LED>>,
}

impl GPUEventLoop {
    pub fn new(
        pipeline: EmblinkenatorPipeline,
        frame_data_buffer: crossbeam::channel::Receiver<FrameData>,
        pipeline_context_buffer: crossbeam::channel::Receiver<PipelineContext>,
        frame_output_buffer: tokio::sync::broadcast::Sender<PipelineFrameOutput>,
    ) -> GPUEventLoop {
        GPUEventLoop {
            state: GPUEventLoopState::Paused,
            command_queue: vec![],
            pipeline,
            frame_data_buffer,
            pipeline_context_buffer,
            frame_output_buffer,
            frame_state: None,
        }
    }

    pub async fn tick(&mut self) {
        self.state = GPUEventLoopState::BeforeFrame;

        // TODO: Any awaited steps here should likely either block directly or be run in a polling mode. May be waiting on upstream WebGPU work.
        'eventloop: loop {
            match self.state {
                GPUEventLoopState::Paused => self.loop_step_paused(),
                GPUEventLoopState::BeforeFrame => self.loop_step_before_frame(),
                GPUEventLoopState::Compute => self.loop_step_compute(),
                GPUEventLoopState::WaitForGPUIdle => self.loop_step_wait_for_gpu_idle(),
                GPUEventLoopState::ReadDataFromGPU => self.loop_step_read_data_from_gpu().await,
                GPUEventLoopState::OutputData => self.loop_step_output_data(),
                GPUEventLoopState::FrameEnd => self.loop_step_frame_end().await,
            }

            for command in &self.command_queue {
                if *command == GPUEventLoopQueue::Exit {
                    break 'eventloop;
                }
            }
        }
    }

    fn loop_step_paused(&mut self) {}

    fn loop_step_before_frame(&mut self) {
        debug!("Before frame");

        self.frame_state = Some(EventLoopFrameState::new());
        let pipeline_context = self.pipeline_context_buffer.recv().unwrap();
        self.pipeline.before_frame(&pipeline_context).unwrap();

        self.next_state();
    }

    fn loop_step_compute(&mut self) {
        let frame_data = self
            .frame_data_buffer
            .recv()
            .expect("Frame data buffer closed");
        debug!("Compute Frame {}", frame_data.frame);

        self.pipeline.compute_frame(&frame_data).unwrap();

        self.next_state();
    }

    fn loop_step_wait_for_gpu_idle(&mut self) {
        debug!("Wait for idle");
        self.pipeline.poll_device();

        self.next_state();
    }

    async fn loop_step_read_data_from_gpu(&mut self) {
        debug!("Read from GPU");
        let data = self.pipeline.read_led_states().await;
        let mut state = self.frame_state.as_ref().unwrap().clone();
        // TODO: This feels nasty
        state.last_frame_state = Some(data);
        self.frame_state.replace(state);

        self.next_state();
    }

    fn loop_step_output_data(&mut self) {
        debug!("Output data");

        if let Some(frame_state) = &self.frame_state.as_ref().unwrap().last_frame_state {
            let output: PipelineFrameOutput = PipelineFrameOutput {
                states: frame_state.states.clone(),
            };

            self.frame_output_buffer.send(output).unwrap();
        }

        self.next_state();
    }

    async fn loop_step_frame_end<'a>(&mut self) {
        debug!("Frame end");

        self.next_state();
    }

    fn next_state(&mut self) {
        self.state = match self.state {
            GPUEventLoopState::Paused => GPUEventLoopState::Paused,
            GPUEventLoopState::BeforeFrame => GPUEventLoopState::Compute,
            GPUEventLoopState::Compute => GPUEventLoopState::WaitForGPUIdle,
            GPUEventLoopState::WaitForGPUIdle => GPUEventLoopState::ReadDataFromGPU,
            GPUEventLoopState::ReadDataFromGPU => GPUEventLoopState::OutputData,
            GPUEventLoopState::OutputData => GPUEventLoopState::FrameEnd,
            GPUEventLoopState::FrameEnd => GPUEventLoopState::BeforeFrame,
        }
    }
}

impl EventLoopFrameState {
    fn new() -> Self {
        EventLoopFrameState {
            last_frame_state: None,
        }
    }
}
