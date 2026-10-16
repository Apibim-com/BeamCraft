// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
//! Offscreen native wgpu renderer. CPU serialization is explicit little endian, no unsafe casts.
use crate::{
    Error,
    camera::View,
    scene::{MAX_OBJECTS, Scene},
};
use beamcraft_kernel::{RectangularPlate, rectangular_plate};
use std::time::{Duration, Instant};
fn error_scopes(device: &wgpu::Device) -> [wgpu::ErrorScopeGuard; 3] {
    [
        device.push_error_scope(wgpu::ErrorFilter::OutOfMemory),
        device.push_error_scope(wgpu::ErrorFilter::Internal),
        device.push_error_scope(wgpu::ErrorFilter::Validation),
    ]
}
fn finish_scopes(scopes: [wgpu::ErrorScopeGuard; 3]) -> Result<(), Error> {
    let mut error = None;
    for scope in scopes.into_iter().rev() {
        if let Some(e) = pollster::block_on(scope.pop()) {
            error = Some(Error::Graphics(e.to_string()));
        }
    }
    match error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// Two equivalent submission strategies, each using one draw call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// One G02 unit box, with translation and positive dimension scale per instance.
    Instanced,
    /// Each object's G02 triangles expanded into one merged vertex buffer.
    Merged,
}
impl Strategy {
    /// Machine-readable strategy name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Instanced => "instanced",
            Self::Merged => "merged",
        }
    }
}
/// Graphics context and actual adapter identity; selection prefers discrete, integrated, then CPU.
pub struct Gpu {
    /// Adapter used, never inferred from the operating system.
    pub adapter: wgpu::Adapter,
    /// Native graphics device.
    pub device: wgpu::Device,
    /// Its command queue.
    pub queue: wgpu::Queue,
}
impl Gpu {
    /// Enumerate native adapters; try device creation in ranked order, including software fallback.
    pub async fn headless() -> Result<Self, Error> {
        let backends = wgpu::Backends::DX12 | wgpu::Backends::VULKAN | wgpu::Backends::METAL;
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = backends;
        let instance = wgpu::Instance::new(desc);
        let mut adapters = instance.enumerate_adapters(backends).await;
        adapters.sort_by_key(|a| match a.get_info().device_type {
            wgpu::DeviceType::DiscreteGpu => 0,
            wgpu::DeviceType::IntegratedGpu => 1,
            wgpu::DeviceType::VirtualGpu => 2,
            wgpu::DeviceType::Other => 3,
            wgpu::DeviceType::Cpu => 4,
        });
        if adapters.is_empty() {
            return Err(Error::NoAdapter);
        }
        let mut errors = Vec::new();
        for adapter in adapters {
            match adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("V01"),
                    ..Default::default()
                })
                .await
            {
                Ok((device, queue)) => {
                    return Ok(Self {
                        adapter,
                        device,
                        queue,
                    });
                }
                Err(e) => errors.push(e.to_string()),
            }
        }
        Err(Error::Graphics(errors.join("; ")))
    }
}
/// Wait for a specific submission, with a finite device timeout.
pub fn wait(device: &wgpu::Device, index: wgpu::SubmissionIndex) -> Result<(), Error> {
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(index),
            timeout: Some(Duration::from_secs(60)),
        })
        .map_err(|e| Error::Graphics(e.to_string()))?;
    Ok(())
}
/// Owned buffers, kept separate from pipeline/texture setup for honest upload timing.
pub struct Uploaded {
    vertex: wgpu::Buffer,
    instance: Option<wgpu::Buffer>,
    strategy: Strategy,
    vertices: u32,
    instances: u32,
    /// Exact vertex/instance bytes sent (excludes fixed 80-byte camera uniform).
    pub bytes: u64,
    /// Serialization time before device allocation/upload.
    pub pack_ms: f64,
    /// Buffer allocation, writes and flush-to-completion time.
    pub upload_ms: f64,
}
fn bytes_vec(capacity: usize) -> Result<Vec<u8>, Error> {
    let mut v = Vec::new();
    v.try_reserve_exact(capacity)
        .map_err(|_| Error::Allocation)?;
    Ok(v)
}
fn push_vertex(bytes: &mut Vec<u8>, position: [f32; 3], normal: [f32; 3]) {
    for x in position.into_iter().chain(normal) {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
}
fn plate_vertices(
    bytes: &mut Vec<u8>,
    plate: &beamcraft_kernel::PlateResult,
    translation: [f32; 3],
) -> Result<(), Error> {
    let mesh = plate.mesh();
    let [tx, ty, tz] = translation;
    for (tri, face) in mesh.triangles().iter().zip(mesh.triangle_faces()) {
        let n = mesh
            .face_normals()
            .get(usize::from(*face))
            .ok_or(Error::Input("kernel face index"))?;
        for i in tri {
            let p = mesh
                .vertices()
                .get(*i as usize)
                .ok_or(Error::Input("kernel vertex index"))?;
            push_vertex(
                bytes,
                [p.x as f32 + tx, p.y as f32 + ty, p.z as f32 + tz],
                [n.x() as f32, n.y() as f32, n.z() as f32],
            );
        }
    }
    Ok(())
}
fn buffer(device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8]) -> wgpu::Buffer {
    let b = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("V01 vertices/instances"),
        size: bytes.len() as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&b, 0, bytes);
    b
}
impl Uploaded {
    /// Serialize the engine meshes or per-instance draw transforms, then upload and wait.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
        strategy: Strategy,
    ) -> Result<Self, Error> {
        let count = scene.objects.len();
        if count == 0 || count > MAX_OBJECTS as usize {
            return Err(Error::Input("upload count"));
        }
        for object in &scene.objects {
            if !object
                .translation
                .iter()
                .all(|x| x.is_finite() && x.abs() <= 1e7)
                || !object
                    .scale
                    .iter()
                    .all(|x| x.is_finite() && *x > 0.0 && *x <= 1e6)
            {
                return Err(Error::Input("non-finite or out-of-range display transform"));
            }
        }
        let start = Instant::now();
        let capacity = if strategy == Strategy::Merged {
            count.checked_mul(864).ok_or(Error::Input("vertex bytes"))?
        } else {
            864
        };
        if capacity as u64 > device.limits().max_buffer_size {
            return Err(Error::Input("adapter buffer limit"));
        }
        let mut vertices = bytes_vec(capacity)?;
        let mut instances = bytes_vec(if strategy == Strategy::Instanced {
            count * 24
        } else {
            0
        })?;
        match strategy {
            Strategy::Instanced => {
                let unit = rectangular_plate(RectangularPlate {
                    width_mm: 1.0,
                    length_mm: 1.0,
                    thickness_mm: 1.0,
                })
                .map_err(Error::Kernel)?;
                plate_vertices(&mut vertices, &unit, [0.0; 3])?;
                for o in &scene.objects {
                    for x in o.translation.into_iter().chain(o.scale) {
                        instances.extend_from_slice(&x.to_le_bytes());
                    }
                }
            }
            Strategy::Merged => {
                for o in &scene.objects {
                    plate_vertices(&mut vertices, &o.plate, o.translation)?;
                }
            }
        }
        let pack_ms = start.elapsed().as_secs_f64() * 1000.0;
        let bytes = (vertices.len() + instances.len()) as u64;
        let start = Instant::now();
        let scopes = error_scopes(device);
        let vertex = buffer(device, queue, &vertices);
        let instance = if instances.is_empty() {
            None
        } else {
            Some(buffer(device, queue, &instances))
        };
        let completion = wait(device, queue.submit([]));
        finish_scopes(scopes)?;
        completion?;
        Ok(Self {
            vertex,
            instance,
            strategy,
            vertices: if strategy == Strategy::Merged {
                count as u32 * 36
            } else {
                36
            },
            instances: if strategy == Strategy::Instanced {
                count as u32
            } else {
                1
            },
            bytes,
            pack_ms,
            upload_ms: start.elapsed().as_secs_f64() * 1000.0,
        })
    }
}
/// Fixed-size offscreen target with depth and 4x MSAA when both formats support it.
pub struct Renderer {
    /// Resolved texture view, suitable for registering with egui.
    pub view: wgpu::TextureView,
    texture: wgpu::Texture,
    depth: wgpu::TextureView,
    msaa: Option<wgpu::TextureView>,
    instanced: wgpu::RenderPipeline,
    merged: wgpu::RenderPipeline,
    camera: wgpu::Buffer,
    bind: wgpu::BindGroup,
    /// Actual sample count used.
    pub samples: u32,
    /// Offscreen width in pixels.
    pub width: u32,
    /// Offscreen height in pixels.
    pub height: u32,
}
impl Renderer {
    /// Create bounded targets and pipelines. Setup is excluded from upload and frame measurements.
    pub fn new(
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Result<Self, Error> {
        if width == 0
            || height == 0
            || width > 2048
            || height > 2048
            || width > device.limits().max_texture_dimension_2d
            || height > device.limits().max_texture_dimension_2d
        {
            return Err(Error::Input("target size 1..=2048"));
        }
        let samples = if [FORMAT, DEPTH].into_iter().all(|f| {
            adapter
                .get_texture_format_features(f)
                .flags
                .sample_count_supported(4)
        }) {
            4
        } else {
            1
        };
        let scopes = error_scopes(device);
        let texture_fn = |format, sample_count, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("V01 target"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let texture = texture_fn(
            FORMAT,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let view = texture.create_view(&Default::default());
        let depth = texture_fn(DEPTH, samples, wgpu::TextureUsages::RENDER_ATTACHMENT)
            .create_view(&Default::default());
        let msaa = if samples == 4 {
            Some(
                texture_fn(FORMAT, 4, wgpu::TextureUsages::RENDER_ATTACHMENT)
                    .create_view(&Default::default()),
            )
        } else {
            None
        };
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("V01 camera"),
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("V01 camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("V01 camera bind"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("V01 pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("V01 original shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("plate.wgsl").into()),
        });
        let attributes = wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3];
        let instance_attributes = wgpu::vertex_attr_array![2=>Float32x3,3=>Float32x3];
        let vertex_layout = || {
            Some(wgpu::VertexBufferLayout {
                array_stride: 24,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attributes,
            })
        };
        let instance_layout = Some(wgpu::VertexBufferLayout {
            array_stride: 24,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &instance_attributes,
        });
        let pipeline = |entry, buffers: &[_]| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    buffers,
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: samples,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("shade"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let instanced = pipeline("instanced", &[vertex_layout(), instance_layout]);
        let merged = pipeline("merged", &[vertex_layout()]);
        finish_scopes(scopes)?;
        Ok(Self {
            view,
            texture,
            depth,
            msaa,
            instanced,
            merged,
            camera,
            bind,
            samples,
            width,
            height,
        })
    }
    /// Encode one lit/depth-tested draw and submit; completion is deliberately a separate measurement.
    pub fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uploaded: &Uploaded,
        camera: View,
    ) -> wgpu::SubmissionIndex {
        queue.write_buffer(&self.camera, 0, &camera.bytes());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("V01 frame"),
        });
        {
            let color = wgpu::RenderPassColorAttachment {
                view: self.msaa.as_ref().unwrap_or(&self.view),
                depth_slice: None,
                resolve_target: self.msaa.as_ref().map(|_| &self.view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.02,
                        g: 0.03,
                        b: 0.05,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("V01 lit plates"),
                color_attachments: &[Some(color)],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(if uploaded.strategy == Strategy::Instanced {
                &self.instanced
            } else {
                &self.merged
            });
            pass.set_bind_group(0, &self.bind, &[]);
            pass.set_vertex_buffer(0, uploaded.vertex.slice(..));
            if let Some(instance) = &uploaded.instance {
                pass.set_vertex_buffer(1, instance.slice(..));
            }
            pass.draw(0..uploaded.vertices, 0..uploaded.instances);
        }
        queue.submit([encoder.finish()])
    }
    /// Read RGBA pixels (row padding retained) for headless validation, never used by the benchmark.
    pub fn readback(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(Vec<u8>, u32), Error> {
        let row = (self.width * 4).div_ceil(256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("V01 readback"),
            size: u64::from(row) * u64::from(self.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        let index = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        buffer.map_async(wgpu::MapMode::Read, .., move |r| {
            let _ = tx.send(r);
        });
        wait(device, index)?;
        rx.recv_timeout(Duration::from_secs(10))
            .map_err(|e| Error::Graphics(e.to_string()))?
            .map_err(|e| Error::Graphics(e.to_string()))?;
        let mapped = buffer
            .get_mapped_range(..)
            .map_err(|e| Error::Graphics(e.to_string()))?;
        let data = mapped.to_vec();
        drop(mapped);
        buffer.unmap();
        Ok((data, row))
    }
}
