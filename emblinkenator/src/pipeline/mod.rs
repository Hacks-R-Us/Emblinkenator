mod compute_device;

use std::{collections::HashMap, convert::TryInto, mem, u64};

use crate::{
    animation::{Animation, AnimationTargetType},
    auxiliary_data::{
        aux_data_consumer_type_is_compatible, aux_data_to_consumer_type, AuxiliaryData,
        AuxiliaryDataTypeConsumer,
    },
    frame::FrameData,
    id::{AnimationId, AuxiliaryId},
    led::LED,
    world::Coord,
};
use compute_device::{build_compute_device, EmblinkenatorComputeDevice};
use log::{debug, error, info, warn};
use strum::IntoEnumIterator;

pub struct EmblinkenatorPipeline {
    state: EmblinkenatorPipelineState,
    leds_per_compute_group: u32,
    compute_device: EmblinkenatorComputeDevice,
    frame_data_buffer: wgpu::Buffer,
    compute_shaders: HashMap<AnimationId, PipelineEntry>,
    auxiliary_buffers: HashMap<AuxiliaryId, PipelineAuxiliary>,
    empty_auxiliary_buffers: HashMap<AuxiliaryDataTypeConsumer, PipelineAuxiliary>,
    current_context: Option<PipelineContext>,
}

#[derive(Clone, Debug)]
pub struct PipelineContext {
    // String -> Fixture/Installation/Group Id
    pub led_positions: HashMap<String, Vec<Coord>>,
    pub num_leds: HashMap<String, u32>,
    pub animations: HashMap<AnimationId, Animation>,
    pub auxiliary_data: HashMap<AuxiliaryId, AuxiliaryData>,
    pub animation_auxiliary_data: HashMap<AnimationId, Vec<AuxiliaryId>>,
}

struct PipelineEntry {
    id: AnimationId,
    target_id: String,
    storage_buffer: wgpu::Buffer,
    staging_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    result_bind_group: wgpu::BindGroup,
    auxiliary_bind_group_layout: wgpu::BindGroupLayout,
    auxiliary_types: Vec<AuxiliaryDataTypeConsumer>,
    positions_data_buffer: wgpu::Buffer,
    num_leds: u32,
    result_size: u64,
    work_group_count: u32,
}

struct PipelineAuxiliary {
    buffer: wgpu::Buffer,
    aux_type: AuxiliaryDataTypeConsumer,
    size: u64,
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
    TargetDoesNotExist(AnimationId, AnimationTargetType),
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
            auxiliary_data: HashMap::new(),
            animation_auxiliary_data: HashMap::new(),
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

        let mut empty_auxiliary_buffers = HashMap::new();

        for aux_type in AuxiliaryDataTypeConsumer::iter() {
            let empty_buffer_vec = aux_type.empty_buffer();
            let empty_buffer = compute_device
                .create_auxiliary_data_buffer_src(aux_type.to_string(), &empty_buffer_vec);
            empty_auxiliary_buffers.insert(
                aux_type.clone(),
                PipelineAuxiliary {
                    aux_type: aux_type.clone(),
                    buffer: empty_buffer,
                    size: empty_buffer_vec.len() as u64,
                },
            );
        }

        EmblinkenatorPipeline {
            state: EmblinkenatorPipelineState::Idle,
            leds_per_compute_group,
            compute_device,
            frame_data_buffer,
            compute_shaders: HashMap::new(),
            auxiliary_buffers: HashMap::new(),
            empty_auxiliary_buffers,
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
        let mut removed_animations: Vec<AnimationId> = vec![];
        let mut added_auxiliaries: Vec<(AuxiliaryId, AuxiliaryData)> = vec![];
        let mut resized_auxiliaries: Vec<(AuxiliaryId, AuxiliaryData)> = vec![];
        let mut removed_auxiliaries: Vec<AuxiliaryId> = vec![];
        if let Some(prev_state) = prev_state {
            for auxiliary in context.auxiliary_data.iter() {
                match prev_state.auxiliary_data.get(auxiliary.0) {
                    Some(previous_auxiliary) => {
                        if previous_auxiliary.size != auxiliary.1.size {
                            info!(
                                "Auxiliary {} changed size from {} to {}",
                                auxiliary.0, previous_auxiliary.size, auxiliary.1.size
                            );
                            resized_auxiliaries.push((auxiliary.0.clone(), auxiliary.1.clone()))
                        }
                    }
                    None => {
                        // New auxiliary
                        info!(
                            "New auxiliary {} with size {}",
                            auxiliary.0, auxiliary.1.size
                        );
                        added_auxiliaries.push((auxiliary.0.clone(), auxiliary.1.clone()));
                    }
                }
            }

            for animation in context.animations.iter() {
                if !prev_state.animations.contains_key(animation.0) {
                    // New animation
                    info!("New animation {}", animation.0);
                    added_animations.push((animation.0.clone(), animation.1.clone()));
                }
            }

            for auxiliary in prev_state.auxiliary_data.iter() {
                if !context.auxiliary_data.contains_key(auxiliary.0) {
                    // Removed auxiliary
                    info!("Removed auxiliary {}", auxiliary.0);
                    removed_auxiliaries.push(auxiliary.0.clone());
                }
            }

            for animation in prev_state.animations.iter() {
                if !context.animations.contains_key(animation.0) {
                    // Removed animation
                    removed_animations.push(animation.0.clone());
                }
            }
        } else {
            added_auxiliaries = context
                .auxiliary_data
                .iter()
                .map(|auxiliary| (auxiliary.0.clone(), auxiliary.1.clone()))
                .collect();

            added_animations = context
                .animations
                .iter()
                .map(|animation| (animation.0.clone(), animation.1.clone()))
                .collect();
        }

        if !removed_auxiliaries.is_empty() {
            for removed_auxiliary in removed_auxiliaries.iter() {
                self.remove_auxiliary(removed_auxiliary);
            }
            self.load_shaders_to_gpu();
        }

        if !resized_auxiliaries.is_empty() {
            for resized_auxiliary in resized_auxiliaries.into_iter() {
                self.remove_auxiliary(&resized_auxiliary.0);
                added_auxiliaries.push(resized_auxiliary);
            }
            self.load_shaders_to_gpu();
        }

        if !added_auxiliaries.is_empty() {
            for auxiliary in added_auxiliaries.into_iter() {
                info!("Adding auxiliary {}", auxiliary.0);
                self.add_auxiliary(auxiliary.0, auxiliary.1);
            }
            self.load_shaders_to_gpu();
        }

        if !removed_animations.is_empty() {
            for removed_animation in removed_animations.iter() {
                self.remove_shader(removed_animation);
            }
        }

        if !added_animations.is_empty() {
            for animation in added_animations.into_iter() {
                info!("Loading animation {}", animation.0);
                if let Err(err) = self.add_shader(context, animation.0, animation.1) {
                    match err {
                        EmblinkenatorPipelineError::WrongState(msg) => panic!(
                            "Pipeline was in wrong state before frame in add_shader: {}",
                            msg
                        ),
                        EmblinkenatorPipelineError::TargetDoesNotExist(animation_id, target) => {
                            panic!(
                                "Tried to add animation {} but target {} does not exist.",
                                animation_id,
                                String::from(target)
                            )
                        }
                        EmblinkenatorPipelineError::NoContext(msg) => error!(
                            "Tried to call add_shader before frame but no context was provided: {}",
                            msg
                        ),
                    }
                }
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
                animation.target,
            ));
        }

        let num_leds = *num_leds.unwrap();

        let result_size =
            3 * u64::from(num_leds) * std::mem::size_of::<u32>() as wgpu::BufferAddress;
        let positions_size = ((num_leds * 3) as usize * mem::size_of::<f32>()) as u64;
        let work_group_count =
            ((num_leds as f32) / (self.leds_per_compute_group as f32)).ceil() as u32;

        debug!("Create shader module");
        let shader = self
            .compute_device
            .create_shader_module(animation.id(), animation.get_shader_str());

        debug!("Create storage buffer");
        let storage_buffer = self
            .compute_device
            .create_storage_buffer(id.unprotect(), result_size);

        debug!("Create staging buffer");
        let staging_buffer = self
            .compute_device
            .create_staging_buffer(id.unprotect(), result_size);

        debug!("Create positions data buffer");
        let positions_data_buffer = self
            .compute_device
            .create_positions_buffer_dest(id.unprotect(), num_leds);

        debug!("Create auxiliaries");
        let auxiliaries = animation.get_auxiliaries();
        let mut auxiliary_bind_group_entries: Vec<wgpu::BindGroupLayoutEntry> = vec![];

        if let Some(auxiliaries) = auxiliaries {
            for _ in auxiliaries.iter() {
                auxiliary_bind_group_entries.push(wgpu::BindGroupLayoutEntry {
                    binding: auxiliary_bind_group_entries.len() as u32,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                });
            }
        }

        let compute_bind_group_entries: Vec<wgpu::BindGroupLayoutEntry> = vec![
            // Frame Info
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (FrameData::num_fields() * mem::size_of::<f32>()) as _,
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
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
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
        let auxiliary_bind_group_layout = self.compute_device.create_bind_group_layout(
            format!("Auxiliary bund group layout: {}", id.unprotect()).as_str(),
            &auxiliary_bind_group_entries,
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
            &auxiliary_bind_group_layout,
        );
        let compute_pipeline = self.compute_device.create_shader_compute_pipeline(
            id.unprotect(),
            compute_pipeline_layout,
            shader,
        );

        self.compute_shaders.insert(
            id.clone(),
            PipelineEntry {
                id,
                target_id,
                storage_buffer,
                staging_buffer,
                compute_pipeline,
                compute_bind_group,
                result_bind_group,
                auxiliary_bind_group_layout,
                positions_data_buffer,
                num_leds,
                work_group_count,
                result_size,
                auxiliary_types: animation.get_auxiliaries().unwrap_or_default(),
            },
        );

        Ok(())
    }

    pub fn remove_shader(&mut self, id: &AnimationId) {
        self.compute_shaders.remove(id);
    }

    pub fn add_auxiliary(&mut self, id: AuxiliaryId, auxiliary: AuxiliaryData) {
        let auxiliary_size =
            u64::from(auxiliary.size) * aux_data_to_consumer_type(&auxiliary.data).mem_size();
        let auxiliary_buffer = self
            .compute_device
            .create_auxiliary_data_buffer_dest(id.unprotect(), auxiliary_size);
        let auxiliary = PipelineAuxiliary {
            buffer: auxiliary_buffer,
            aux_type: aux_data_to_consumer_type(&auxiliary.data),
            size: auxiliary_size,
        };

        self.auxiliary_buffers.insert(id, auxiliary);
    }

    pub fn remove_auxiliary(&mut self, id: &AuxiliaryId) {
        self.auxiliary_buffers.remove(id);
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

        let new_frame_data_vec: Vec<f32> = [
            frame_data.frame as f32,
            frame_data.frame_numerator as f32,
            frame_data.frame_denominator as f32,
            frame_data.seconds_elapsed,
            frame_data.whole_seconds_elapsed as f32,
        ]
        .to_vec();

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

        // Copy auxiliary data
        for (auxiliary_id, auxiliary) in context.auxiliary_data.iter() {
            if let Some(aux_data_dest) = self.auxiliary_buffers.get(auxiliary_id) {
                let new_aux_data = auxiliary.data.to_data_buffer();
                let aux_data_src = self
                    .compute_device
                    .create_auxiliary_data_buffer_src(auxiliary_id.unprotect(), &new_aux_data);

                debug!(
                    "Auxilairy {} size {}",
                    auxiliary_id,
                    new_aux_data.len() * mem::size_of::<u8>()
                );
                command_encoder.copy_buffer_to_buffer(
                    &aux_data_src,
                    0,
                    &aux_data_dest.buffer,
                    0,
                    (new_aux_data.len() * mem::size_of::<u8>()) as u64,
                )
            }
        }

        for (_id, shader) in self.compute_shaders.iter() {
            let led_positions = context.led_positions.get(&shader.target_id);

            if led_positions.is_none() {
                warn!("Shader {} does not have any LED positions", shader.id);
                continue;
            }

            let mut auxiliary_group_entries: Vec<wgpu::BindGroupEntry> = vec![];

            let required_auxiliaries = shader.auxiliary_types.clone(); // TODO: Can we just reference this property directly?
            let mapped_auxiliaries = context
                .animation_auxiliary_data
                .get(&shader.id)
                .cloned()
                .unwrap_or_default();
            let mut invalid_auxiliaries: Vec<(u32, AuxiliaryDataTypeConsumer)> = vec![];

            for (index, required_aux) in required_auxiliaries.iter().enumerate() {
                let mapped_aux_id = mapped_auxiliaries.get(index);
                if mapped_aux_id.is_none() {
                    debug!(
                        "Auxiliary {} is not mapped for shader {}, an empty buffer will be created",
                        index, shader.id
                    );
                    invalid_auxiliaries.push((index as u32, required_aux.clone()));
                    continue;
                }
                let mapped_aux_id = mapped_aux_id.unwrap();

                let aux = self.auxiliary_buffers.get(mapped_aux_id);
                if aux.is_none() {
                    error!("Auxiliary {} is mapped for shader {} but does not exist in the current context, an empty buffer will be used", index, shader.id);
                    invalid_auxiliaries.push((index as u32, required_aux.clone()));
                    continue;
                }
                let aux = aux.unwrap();

                if !aux_data_consumer_type_is_compatible(&aux.aux_type, required_aux) {
                    error!("Auxiliary {} is mapped for shader {} but is not the right type, an empty buffer will be used", index, shader.id);
                    invalid_auxiliaries.push((index as u32, required_aux.clone()));
                    continue;
                }

                auxiliary_group_entries.push(wgpu::BindGroupEntry {
                    binding: index as u32,
                    resource: aux.buffer.as_entire_binding(),
                });
            }

            for (index, missing_auxiliary_type) in invalid_auxiliaries {
                let empty_auxiliary_buffer = self
                    .empty_auxiliary_buffers
                    .get(&missing_auxiliary_type)
                    .unwrap_or_else(|| {
                        panic!(
                            "Empty buffer should exist for auxilary type {}",
                            missing_auxiliary_type
                        )
                    });
                auxiliary_group_entries.push(wgpu::BindGroupEntry {
                    binding: index,
                    resource: empty_auxiliary_buffer.buffer.as_entire_binding(),
                })
            }

            let auxiliaries_bind_group = self.compute_device.create_bind_group(
                format!("Auxiliary bind group: {}", shader.id.unprotect()).as_str(),
                &shader.auxiliary_bind_group_layout,
                &auxiliary_group_entries,
            );

            let led_positions_flat: Vec<f32> = led_positions
                .unwrap()
                .iter()
                .flat_map(|p| p.flat())
                .collect();

            let led_positions_buffer = self.compute_device.create_positions_buffer_src(
                format!("{} {}", shader.id.unprotect(), frame_data.frame).to_string(),
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

            command_encoder.push_debug_group(
                format!("Compute pattern state {}", shader.id.unprotect()).as_str(),
            );
            {
                // Compute pass
                let mut cpass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some(&shader.id.unprotect()),
                });
                cpass.set_pipeline(&shader.compute_pipeline);
                cpass.set_bind_group(0, &shader.compute_bind_group, &[]);
                cpass.set_bind_group(1, &shader.result_bind_group, &[]);
                cpass.set_bind_group(2, &auxiliaries_bind_group, &[]);
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

        for (_id, shader) in self.compute_shaders.iter() {
            let buffer_slice = shader.staging_buffer.slice(..);
            let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

            // Need buffer to be mapped
            self.poll_device();

            // Awaits until `buffer_future` can be read from
            if let Ok(()) = buffer_future.await {
                // Gets contents of buffer
                let data = buffer_slice.get_mapped_range();
                // Since contents are got in bytes, this converts these bytes back to f32
                let result: Vec<f32> = data
                    .chunks_exact(4)
                    .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
                    .collect();

                let state: Vec<LED> = result.chunks(3).map(LED::from).collect();

                if state.len() == shader.num_leds as usize {
                    states.insert(shader.id.unprotect(), state);
                } else {
                    panic!(
                        "Shader {} did not return enough LED states. Expected {} Got {}",
                        shader.id.unprotect(),
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
