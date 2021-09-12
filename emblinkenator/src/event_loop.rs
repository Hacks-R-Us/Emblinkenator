use std::u128;

use log::{debug, info};
use tokio::time::{self, Instant, Interval};

use crate::{
    frame::FrameData,
    frame_resolver::FrameResolverInput,
    pipeline::{ComputeOutput, EmblinkenatorPipeline, PipelineContext},
};

pub struct GPUEventLoop {
    state: GPUEventLoopState,
    command_queue: Vec<GPUEventLoopQueue>,
    pipeline: EmblinkenatorPipeline,
    frame_data: FrameData,
    clock_frame: Interval,
    pipeline_context_buffer: crossbeam::channel::Receiver<PipelineContext>,
    frame_output_buffer: crossbeam::channel::Sender<FrameResolverInput>,
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
    WaitForSync,
}

#[derive(Debug, PartialEq)]
pub enum GPUEventLoopQueue {
    Exit,
}

#[derive(Clone)]
struct EventLoopFrameState {
    pub last_frame_state: Option<ComputeOutput>,
    pub frame_start: Instant,
}

impl GPUEventLoop {
    pub fn new(
        pipeline: EmblinkenatorPipeline,
        frame_rate: u32,
        pipeline_context_buffer: crossbeam::channel::Receiver<PipelineContext>,
        frame_output_buffer: crossbeam::channel::Sender<FrameResolverInput>,
    ) -> GPUEventLoop {
        // TODO: Dodgy!
        let frame_time: u64 = u64::from(1000 / frame_rate);
        let clock_frame = time::interval(time::Duration::from_millis(frame_time));

        GPUEventLoop {
            state: GPUEventLoopState::Paused,
            command_queue: vec![],
            pipeline,
            frame_data: FrameData {
                frame: 0,
                frame_rate,
            },
            clock_frame,
            pipeline_context_buffer,
            frame_output_buffer,
            frame_state: None,
        }
    }

    pub async fn run(&mut self) {
        self.state = GPUEventLoopState::BeforeFrame;
        self.frame_data.frame = 1;

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
                GPUEventLoopState::WaitForSync => self.loop_step_wait_for_sync().await,
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
        debug!("Compute Frame {}", self.frame_data.frame);
        self.pipeline.compute_frame(&self.frame_data).unwrap();

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

    async fn loop_step_wait_for_sync(&mut self) {
        debug!("Wait for sync");
        self.clock_frame.tick().await;

        self.next_state();
    }

    fn loop_step_output_data(&mut self) {
        debug!("Output data");

        if let Some(frame_state) = &self.frame_state.as_ref().unwrap().last_frame_state {
            let output: FrameResolverInput = FrameResolverInput {
                states: frame_state.states.clone(),
            };

            self.frame_output_buffer.send(output).unwrap();
        }

        self.next_state();
    }

    async fn loop_step_frame_end<'a>(&mut self) {
        debug!("Frame end");

        if let Some(prev_state) = self.frame_state.as_ref() {
            // TODO: Dodgy!
            let frame_time: u128 = u128::from(1000 / self.frame_data.frame_rate);
            let elapsed_time = Instant::now()
                .duration_since(prev_state.frame_start)
                .as_millis();

            if elapsed_time > frame_time {
                info!(
                    "Running late by {}ms (Took {}ms)",
                    elapsed_time - frame_time,
                    elapsed_time
                );
            }
        }

        self.frame_data.frame += 1;
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
            GPUEventLoopState::FrameEnd => GPUEventLoopState::WaitForSync,
            GPUEventLoopState::WaitForSync => GPUEventLoopState::BeforeFrame,
        }
    }
}

impl EventLoopFrameState {
    fn new() -> Self {
        EventLoopFrameState {
            last_frame_state: None,
            frame_start: Instant::now(),
        }
    }
}
