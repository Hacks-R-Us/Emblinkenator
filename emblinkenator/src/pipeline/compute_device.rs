use std::{borrow::Cow, mem};

use log::info;
use wgpu::util::DeviceExt;

use crate::id::AnimationId;

pub struct EmblinkenatorComputeDevice {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl EmblinkenatorComputeDevice {
    fn new(device: wgpu::Device, queue: wgpu::Queue) -> EmblinkenatorComputeDevice {
        EmblinkenatorComputeDevice { device, queue }
    }

    pub fn create_shader_module(&self, id: AnimationId, shader: String) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: Some(&id.unprotect()),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader.as_str())),
            })
    }

    pub fn create_zeros_buffer(&self, num_leds: u32) -> wgpu::Buffer {
        let zeros: Vec<u32> = vec![0; num_leds as usize * 3];

        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Empty Buffer"),
                contents: bytemuck::cast_slice(&zeros),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            })
    }

    pub fn create_staging_buffer(&self, id: String, result_size: u64) -> wgpu::Buffer {
        // Staging buffer -> "Output" from GPU.
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(format!("Staging Buffer: {}", id).as_str()),
            size: result_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn create_storage_buffer(&self, id: String, result_size: u64) -> wgpu::Buffer {
        // Storage buffer -> GPU internal buffer for shader to write to.
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(format!("Storage Buffer: {}", id).as_str()),
            size: result_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    pub fn create_positions_buffer_dest(&self, id: String, num_leds: u32) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(format!("Positions Data Buffer Src: {}", id).as_str()),
            size: ((num_leds * 3) as usize * mem::size_of::<f32>()) as _,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn create_frame_data_buffer_dest(&self, id: String) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(format!("Frame Data Buffer Dest: {}", id).as_str()),
            size: (2 * mem::size_of::<f32>()) as _, // TODO: 2 should be number of frame fields.
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn create_auxiliary_data_buffer_dest(&self, id: String, size: u64) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(format!("Aux Data Buffer Dest: {}", id).as_str()),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn create_positions_buffer_src(&self, id: String, positions: &[f32]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(format!("Positions Data Buffer Src: {}", id).as_str()),
                contents: bytemuck::cast_slice(positions),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            })
    }

    pub fn create_frame_data_buffer_src(&self, id: String, frame_data: &[f32]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(format!("Frame Data Buffer Src: {}", id).as_str()),
                contents: bytemuck::cast_slice(frame_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            })
    }

    pub fn create_auxiliary_data_buffer_src(&self, id: String, aux_data: &[u8]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Auxiliary Data Buffer Src: {}", id)),
                contents: aux_data,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            })
    }

    pub fn create_bind_group_layout(
        &self,
        label: &str,
        compute_bind_entries: &[wgpu::BindGroupLayoutEntry],
    ) -> wgpu::BindGroupLayout {
        self.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: compute_bind_entries,
                label: Some(label),
            })
    }

    pub fn create_bind_group(
        &self,
        label: &str,
        layout: &wgpu::BindGroupLayout,
        entries: &[wgpu::BindGroupEntry],
    ) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries,
            label: Some(label),
        })
    }

    pub fn create_shader_compute_pipeline_layout(
        &self,
        _id: String,
        compute_bind_group_layout: &wgpu::BindGroupLayout,
        result_bind_group_layout: &wgpu::BindGroupLayout,
        auxiliary_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::PipelineLayout {
        self.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute"),
                bind_group_layouts: &[
                    compute_bind_group_layout,
                    result_bind_group_layout,
                    auxiliary_bind_group_layout,
                ],
                push_constant_ranges: &[],
            })
    }

    pub fn create_shader_compute_pipeline(
        &self,
        id: String,
        layout: wgpu::PipelineLayout,
        shader: wgpu::ShaderModule,
    ) -> wgpu::ComputePipeline {
        self.device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(format!("Compute pipeline: {}", id).as_str()),
                layout: Some(&layout),
                module: &shader,
                entry_point: "main",
            })
    }

    pub fn create_compute_command_encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute command encoder"),
            })
    }

    pub fn do_work(&self, encoder: wgpu::CommandBuffer) {
        self.queue.submit(Some(encoder));
    }

    pub fn submit_shader(&self) {
        self.queue.submit(None);
    }

    pub fn poll_device(&self) {
        self.device.poll(wgpu::Maintain::Wait);
    }
}

// TODO: Turn this into a builder
pub async fn build_compute_device() -> EmblinkenatorComputeDevice {
    let backend = if let Ok(backend) = std::env::var("WGPU_BACKEND") {
        match backend.to_lowercase().as_str() {
            "vulkan" => wgpu::Backends::VULKAN,
            "metal" => wgpu::Backends::METAL,
            "dx12" => wgpu::Backends::DX12,
            "dx11" => wgpu::Backends::DX11,
            "gl" => wgpu::Backends::GL,
            "webgpu" => wgpu::Backends::BROWSER_WEBGPU,
            other => panic!("Unknown backend: {}", other),
        }
    } else {
        wgpu::Backends::PRIMARY
    };
    let power_preference = if let Ok(power_preference) = std::env::var("WGPU_POWER_PREF") {
        match power_preference.to_lowercase().as_str() {
            "low" => wgpu::PowerPreference::LowPower,
            "high" => wgpu::PowerPreference::HighPerformance,
            other => panic!("Unknown power preference: {}", other),
        }
    } else {
        wgpu::PowerPreference::default()
    };

    let instance = wgpu::Instance::new(backend);
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .expect("No suitable GPU adapters found on the system!");

    let adapter_info = adapter.get_info();
    info!("Using {} ({:?})", adapter_info.name, adapter_info.backend);

    let trace_dir = std::env::var("WGPU_TRACE");
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            trace_dir.ok().as_ref().map(std::path::Path::new),
        )
        .await
        .expect("Unable to find a suitable GPU adapter!");

    EmblinkenatorComputeDevice::new(device, queue)
}
