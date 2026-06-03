use std::sync::Arc;
use wgpu::*;
use winit::{
    window::{Window},
};
use crate::camera::CameraUniform;

pub struct GPUState {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: RenderPipeline,
    compute_pipeline: ComputePipeline,
    compute_texture: Texture,
    compute_bgl: BindGroupLayout,
    render_bgl: BindGroupLayout,
    compute_bind_group: BindGroup,
    render_bind_group: BindGroup,
    sampler: Sampler,
    pub camera_buffer: Buffer,
}

impl GPUState {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = Instance::new(InstanceDescriptor {
            backends: if cfg!(target_arch = "wasm32") {
                Backends::BROWSER_WEBGPU
            } else {
                Backends::all()
            },
            display: None,
            flags: Default::default(),
            backend_options: Default::default(),
            memory_budget_thresholds: Default::default(),
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: if cfg!(target_arch = "wasm32") {
                    PowerPreference::None
                } else {
                    PowerPreference::HighPerformance
                },
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("Main Device"),
                required_features: Features::empty(),
                required_limits: if cfg!(target_arch = "wasm32") {
                    Limits::downlevel_webgl2_defaults()
                } else {
                    Limits::default()
                },
                experimental_features: Default::default(),
                memory_hints: Default::default(),
                trace: Default::default(),
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let compute_texture = device.create_texture(&TextureDescriptor {
            label: Some("Compute Output"),
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: size_of::<CameraUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let compute_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Compute BGL"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: TextureFormat::Rgba8Unorm,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }]
        });

        let render_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Render BGL"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ]
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Texture Sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let texture_view = compute_texture.create_view(&Default::default());

        let compute_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &compute_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
        });

        let render_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &render_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        let compute_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/compute.wgsl").into()),
        });

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[Some(&compute_bgl)],
                immediate_size: 0
            })),
            module: &compute_shader,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let render_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/fullscreen.wgsl").into()),
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(&render_bgl)],
                immediate_size: 0
            })),
            vertex: VertexState {
                module: &render_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default()
            },
            fragment: Some(FragmentState {
                module: &render_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            surface, device, queue, config, size,
            render_pipeline, compute_pipeline,
            compute_texture, compute_bgl, render_bgl,
            compute_bind_group, render_bind_group,
            sampler, camera_buffer
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.compute_texture = self.device.create_texture(&TextureDescriptor {
                label: Some("Compute Output"),
                size: Extent3d {
                    width: new_size.width,
                    height: new_size.height,
                    depth_or_array_layers: 1
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let view = self.compute_texture.create_view(&Default::default());

            self.compute_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
                label: Some("Compute Bind Group"),
                layout: &self.compute_bgl,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: self.camera_buffer.as_entire_binding(),
                    }
                ],
            });

            self.render_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
                label: Some("Render Bind Group"),
                layout: &self.render_bgl,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
        }
    }

    pub fn render(&mut self) {
        let surface_texture = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(st) => st,
            CurrentSurfaceTexture::Suboptimal(st) => st,
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                let size = self.size;
                self.resize(size);
                return;
            }
            CurrentSurfaceTexture::Validation => {
                log::error!("Surface validation error");
                return;
            },
            CurrentSurfaceTexture::Occluded | CurrentSurfaceTexture::Timeout => return,
        };

        let view = surface_texture.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(
            &CommandEncoderDescriptor { label: Some("Render Encoder"), }
        );

        {
            let mut compute_pass = encoder.begin_compute_pass(
                &ComputePassDescriptor { label: Some("Compute Pass"), timestamp_writes: None }
            );
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

            let wg_x = (self.size.width + 7) / 8;
            let wg_y = (self.size.height + 7) / 8;
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }

    pub fn update_camera(&self, uniform: &CameraUniform) {
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(uniform),
        );
    }
}