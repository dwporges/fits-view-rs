use crate::image::image::FitsData;
use crate::image::render::{FitsGpuResources, ShaderUniforms};
use eframe::wgpu;
use eframe::wgpu::util::DeviceExt;
use std::sync::Arc;

pub fn setup_wgpu_resources(
    device: &wgpu::Device,
    target_format: &wgpu::TextureFormat,
    shader_source: &str,
) -> FitsGpuResources {
    let uniforms = ShaderUniforms {
        bp: 0.0,
        wp: 1.0,
        pan: [0.0, 0.0],
        zoom: 1.0,
        rotation: 0.0,
        aspect_scale: [1.0, 1.0],
        bias: 0.5,
        contrast: 1.0,
        posterize_levels: 8.0,
        scaling_mode: 3,
        recolor_mode: 0,
        invert: 0,
        _pad0: 0.0,
        _pad1: 0.0,
    };

    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("FITS Uniform Buffer"),
        contents: bytemuck::cast_slice(&[uniforms]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("FITS Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                count: None,
            },
        ],
    });

    let size = wgpu::Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("FITS Raw Data Texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("FITS Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("FITS Pipeline Layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("FITS Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("FITS Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: *target_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    FitsGpuResources {
        pipeline,
        bind_group,
        uniform_buffer,
        bind_group_layout,
        sampler,
        texture,
        current_slice: 0,
        image_data: None,
        bscale: 1.0,
        bzero: 0.0,
        width: 1,
        height: 1,
    }
}

pub fn update_wgpu_texture(
    wgpu_state: &eframe::egui_wgpu::RenderState,
    width: usize,
    height: usize,
    bscale: f64,
    bzero: f64,
    image_data_opt: Option<Arc<FitsData>>,
) {
    let device = &wgpu_state.device;
    let queue = &wgpu_state.queue;

    let texture_width = width.max(1) as u32;
    let texture_height = height.max(1) as u32;
    let size = wgpu::Extent3d {
        width: texture_width,
        height: texture_height,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("FITS Raw Data Texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    let plane_size = width * height;

    if let Some(data) = &image_data_opt {
        let initial_slice = data.get_f32_slice(0, plane_size, bscale, bzero);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&initial_slice),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * texture_width),
                rows_per_image: Some(texture_height),
            },
            size,
        );
    } else {
        let zero: [f32; 1] = [0.0];
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&zero),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            size,
        );
    }

    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let mut renderer = wgpu_state.renderer.write();
    let res = renderer
        .callback_resources
        .get_mut::<FitsGpuResources>()
        .unwrap();

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("FITS Bind Group"),
        layout: &res.bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: res.uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&res.sampler),
            },
        ],
    });

    res.texture = texture;
    res.bind_group = bind_group;
    res.image_data = image_data_opt;
    res.bscale = bscale;
    res.bzero = bzero;
    res.width = texture_width;
    res.height = texture_height;
    res.current_slice = 0;
}

pub fn recompile_wgpu_shader(
    wgpu_state: &eframe::egui_wgpu::RenderState,
    shader_source: String,
) -> Result<(), String> {
    let device = &wgpu_state.device;

    let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("FITS Custom Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    let mut renderer = wgpu_state.renderer.write();
    let bind_group_layout = renderer
        .callback_resources
        .get::<FitsGpuResources>()
        .unwrap()
        .bind_group_layout
        .clone();

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("FITS Pipeline Layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });

    let new_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("FITS Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu_state.target_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    if let Some(err) = pollster::block_on(error_scope.pop()) {
        return Err(err.to_string());
    }

    renderer
        .callback_resources
        .get_mut::<FitsGpuResources>()
        .unwrap()
        .pipeline = new_pipeline;
    Ok(())
}
