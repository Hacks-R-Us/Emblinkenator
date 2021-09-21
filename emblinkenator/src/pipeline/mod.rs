mod compute_device;

use std::{collections::HashMap, convert::TryInto, mem, u64};

use crate::{animation::{Animation, AnimationTargetType}, auxiliary_data::AuxiliaryDataType, frame::FrameData, id::{AnimationId, AuxiliaryId}, led::LED, world::Coord};
use compute_device::{build_compute_device, EmblinkenatorComputeDevice};

pub struct EmblinkenatorPipeline {
    state: EmblinkenatorPipelineState,
    leds_per_compute_group: u32,
    compute_device: EmblinkenatorComputeDevice,
    frame_data_buffer: wgpu::Buffer,
    compute_shaders: Vec<PipelineEntry>,
    current_context: Option<PipelineContext>,
}

#[derive(Clone)]
pub struct PipelineContext {
    // String -> Fixture/Installation/Group Id
    pub led_positions: HashMap<String, Vec<Coord>>,
    pub num_leds: HashMap<String, u32>,
    pub animations: HashMap<AnimationId, Animation>,
    pub auxiliary_data: HashMap<AnimationId, Vec<AuxiliaryDataType>>
}

struct PipelineEntry {
    id: String,
    target_id: String,
    storage_buffer: wgpu::Buffer,
    staging_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    result_bind_group: wgpu::BindGroup,
    positions_data_buffer: wgpu::Buffer,
    num_leds: u32,
    result_size: u64,
    work_group_count: u32,
}

#[derive(Clone, Debug)]
pub struct ComputeOutput {
    pub states: HashMap<String, Vec<LED>>,
}

#[derive(Debug, PartialEq)]
enum EmblinkenatorPipelineState {
    Idle,
    Computing,
}

#[derive(Debug, PartialEq)]
pub enum EmblinkenatorPipelineError {
    WrongState(String),
    TargetDoesNotExist(AnimationId),
    NoContext(String),
}

// TODO: Turn into builder
pub async fn build_pipeline(leds_per_compute_group: u32) -> EmblinkenatorPipeline {
    let compute_device = build_compute_device().await;
    EmblinkenatorPipeline::new(leds_per_compute_group, compute_device)
}

impl PipelineContext {
    pub fn new() -> Self {
        PipelineContext {
            num_leds: HashMap::new(),
            led_positions: HashMap::new(),
            animations: HashMap::new(),
            auxiliary_data: HashMap::new()
        }
    }
}

impl EmblinkenatorPipeline {
    pub fn new(
        leds_per_compute_group: u32,
        compute_device: EmblinkenatorComputeDevice,
    ) -> EmblinkenatorPipeline {
        let frame_data_buffer =
            compute_device.create_frame_data_buffer_dest("pipeline".to_string());

        EmblinkenatorPipeline {
            state: EmblinkenatorPipelineState::Idle,
            leds_per_compute_group,
            compute_device,
            frame_data_buffer,
            compute_shaders: vec![],
            current_context: None,
        }
    }

    pub fn before_frame(
        &mut self,
        context: &PipelineContext,
    ) -> Result<(), EmblinkenatorPipelineError> {
        if self.state != EmblinkenatorPipelineState::Idle {
            return Err(EmblinkenatorPipelineError::WrongState(
                "Pipeline must be in IDLE state in order to call before_frame".to_string(),
            ));
        }

        let prev_state = self.current_context.replace(context.clone());
        let mut added_animations: Vec<(AnimationId, Animation)> = vec![];
        if let Some(prev_state) = prev_state {
            for animation in context.animations.iter() {
                if !prev_state.animations.contains_key(animation.0) {
                    // New animation
                    added_animations.push((animation.0.clone(), animation.1.clone()));
                }
            }

            for animation in prev_state.animations.iter() {
                if !context.animations.contains_key(animation.0) {
                    // Removed animation
                    // TODO
                }
            }
        } else {
            added_animations = context
                .animations
                .iter()
                .map(|animation| (animation.0.clone(), animation.1.clone()))
                .collect();
        }

        if !added_animations.is_empty() {
            for animation in added_animations.into_iter() {
                // TODO: Better error handling, should bundle errors
                self.add_shader(context, animation.0, animation.1)?;
            }
            self.load_shaders_to_gpu();
        }

        Ok(())
    }

    pub fn add_shader(
        &mut self,
        context: &PipelineContext,
        id: AnimationId,
        animation: Animation,
    ) -> Result<(), EmblinkenatorPipelineError> {
        if self.state != EmblinkenatorPipelineState::Idle {
            return Err(EmblinkenatorPipelineError::WrongState(
                "Pipeline must be in IDLE state to add a new shader".to_string(),
            ));
        }

        let target_id = match &animation.target {
            AnimationTargetType::Fixture(id) => id.unprotect(),
            AnimationTargetType::Installation(id) => id.unprotect(),
            AnimationTargetType::Group(id) => id.unprotect(),
        };

        let num_leds = context.num_leds.get(&target_id);

        if num_leds.is_none() {
            return Err(EmblinkenatorPipelineError::TargetDoesNotExist(
                animation.id(),
            ));
        }

        let num_leds = *num_leds.unwrap();

        let result_size = 3 * u64::from(num_leds) * std::mem::size_of::<u32>() as wgpu::BufferAddress;
        let positions_size = ((num_leds * 3) as usize * mem::size_of::<f32>()) as u64;
        let work_group_count =
            ((num_leds as f32) / (self.leds_per_compute_group as f32)).ceil() as u32;

        let shader = self
            .compute_device
            .create_shader_module(animation.get_shader_str());

        let storage_buffer = self
            .compute_device
            .create_storage_buffer(id.unprotect(), result_size);
        let staging_buffer = self
            .compute_device
            .create_staging_buffer(id.unprotect(), result_size);
        let positions_data_buffer = self
            .compute_device
            .create_positions_buffer_dest(id.unprotect(), num_leds);

        let compute_bind_group_entries: Vec<wgpu::BindGroupLayoutEntry> = vec![
            // Frame Info
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (2 * mem::size_of::<f32>()) as _, // TODO: Replace 2 with something that tracks with API changes
                    ),
                },
                count: None,
            },
            // LED Positions
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(positions_size),
                },
                count: None,
            },
        ];

        let result_bind_group_entries: Vec<wgpu::BindGroupLayoutEntry> =
            vec![wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage {
                        read_only: false,
                    },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(result_size),
                },
                count: None,
            }];

        let compute_group_entries: Vec<wgpu::BindGroupEntry> = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: self.frame_data_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: positions_data_buffer.as_entire_binding(),
            },
        ];

        let result_group_entries: Vec<wgpu::BindGroupEntry> = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: storage_buffer.as_entire_binding(),
        }];

        let result_bind_group_layout = self.compute_device.create_bind_group_layout(
            format!("Result bind group layout: {}", id.unprotect()).as_str(),
            &result_bind_group_entries,
        );
        let compute_bind_group_layout = self.compute_device.create_bind_group_layout(
            format!("Compute bind group layout: {}", id.unprotect()).as_str(),
            &compute_bind_group_entries,
        );
        let result_bind_group = self.compute_device.create_bind_group(
            format!("Result bind group: {}", id.unprotect()).as_str(),
            &result_bind_group_layout,
            &result_group_entries,
        );
        let compute_bind_group = self.compute_device.create_bind_group(
            format!("Compute bind group: {}", id.unprotect()).as_str(),
            &compute_bind_group_layout,
            &compute_group_entries,
        );
        let compute_pipeline_layout = self.compute_device.create_shader_compute_pipeline_layout(
            id.unprotect(),
            &compute_bind_group_layout,
            &result_bind_group_layout,
        );
        let compute_pipeline = self.compute_device.create_shader_compute_pipeline(
            id.unprotect(),
            compute_pipeline_layout,
            shader,
        );

        self.compute_shaders.push(PipelineEntry {
            id: id.unprotect(),
            target_id,
            storage_buffer,
            staging_buffer,
            compute_pipeline,
            compute_bind_group,
            result_bind_group,
            positions_data_buffer,
            num_leds,
            work_group_count,
            result_size,
        });

        Ok(())
    }

    pub fn load_shaders_to_gpu(&self) {
        self.compute_device.submit_shader();
        self.compute_device.poll_device();
    }

    pub fn compute_frame(
        &mut self,
        frame_data: &FrameData,
    ) -> Result<(), EmblinkenatorPipelineError> {
        if self.state != EmblinkenatorPipelineState::Idle {
            return Err(EmblinkenatorPipelineError::WrongState(
                "Pipeline must be in IDLE state to start frame compute".to_string(),
            ));
        }

        if self.current_context.is_none() {
            return Err(EmblinkenatorPipelineError::NoContext(
                "Pipeline does not have a context set".to_string(),
            ));
        }
        let context = self.current_context.as_ref().unwrap();

        let mut command_encoder = self.compute_device.create_compute_command_encoder();

        let new_frame_data_vec: Vec<f32> =
            [frame_data.frame as f32, frame_data.frame_rate as f32].to_vec();

        let frame_data_buffer = self
            .compute_device
            .create_frame_data_buffer_src(format!("{}", frame_data.frame), &new_frame_data_vec);

        // Copy frame data
        command_encoder.copy_buffer_to_buffer(
            &frame_data_buffer,
            0,
            &self.frame_data_buffer,
            0,
            (new_frame_data_vec.len() * mem::size_of::<f32>()) as u64,
        );

        for shader in &self.compute_shaders {
            let led_positions = context.led_positions.get(&shader.target_id);

            if led_positions.is_none() {
                continue;
            }

            let led_positions_flat: Vec<f32> = led_positions
                .unwrap()
                .iter()
                .flat_map(|p| p.flat())
                .collect();

            let led_positions_buffer = self.compute_device.create_positions_buffer_src(
                format!("{} {}", shader.id, frame_data.frame).to_string(),
                &led_positions_flat,
            );
            let zeros_buffer = self.compute_device.create_zeros_buffer(shader.num_leds);

            // Write 0s to the result buffer
            command_encoder.copy_buffer_to_buffer(
                &zeros_buffer,
                0,
                &shader.storage_buffer,
                0,
                shader.result_size,
            );
            command_encoder.copy_buffer_to_buffer(
                &led_positions_buffer,
                0,
                &shader.positions_data_buffer,
                0,
                (led_positions_flat.len() * mem::size_of::<f32>()) as u64,
            );

            command_encoder
                .push_debug_group(format!("Compute pattern state {}", shader.id).as_str());
            {
                // Compute pass
                let mut cpass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some(shader.id.as_str()),
                });
                cpass.set_pipeline(&shader.compute_pipeline);
                cpass.set_bind_group(0, &shader.compute_bind_group, &[]);
                cpass.set_bind_group(1, &shader.result_bind_group, &[]);
                cpass.dispatch(shader.work_group_count, 1, 1);
            }
            command_encoder.pop_debug_group();
            command_encoder.copy_buffer_to_buffer(
                &shader.storage_buffer,
                0,
                &shader.staging_buffer,
                0,
                shader.result_size,
            );
        }

        // Start compute
        self.compute_device.do_work(command_encoder.finish());

        self.state = EmblinkenatorPipelineState::Computing;

        Ok(())
    }

    pub fn poll_device(&self) {
        self.compute_device.poll_device();
    }

    pub async fn read_led_states(&mut self) -> ComputeOutput {
        let mut states: HashMap<String, Vec<LED>> = HashMap::new();

        for shader in &self.compute_shaders {
            let buffer_slice = shader.staging_buffer.slice(..);
            let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

            // Need buffer to be mapped
            self.poll_device();

            // Awaits until `buffer_future` can be read from
            if let Ok(()) = buffer_future.await {
                // Gets contents of buffer
                let data = buffer_slice.get_mapped_range();
                // Since contents are got in bytes, this converts these bytes back to u32
                let result: Vec<u32> = data
                    .chunks_exact(4)
                    .map(|b| u32::from_ne_bytes(b.try_into().unwrap()))
                    .collect();

                let state: Vec<LED> = result.chunks(3).map(LED::from).collect();

                if state.len() == shader.num_leds as usize {
                    states.insert(shader.id.clone(), state);
                } else {
                    panic!(
                        "Shader {} did not return enough LED states. Expected {} Got {}",
                        shader.id,
                        shader.num_leds,
                        state.len()
                    );
                }

                // With the current interface, we have to make sure all mapped views are
                // dropped before we unmap the buffer.
                drop(data);
                shader.staging_buffer.unmap();
            } else {
                // TODO: handle this
                panic!("Failed to read LED data from GPU")
            }
        }

        self.state = EmblinkenatorPipelineState::Idle;

        ComputeOutput { states }
    }
}
