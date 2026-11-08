//! Mods' sky layers (OptiFine's custom skies): textured cubes around the camera, each a cube's
//! six faces in OptiFine's 3 by 2 grid, drawn after the sky's gradient and sunrise and before
//! the sun, moon and stars, each with one of OptiFine's blends.
//!
//! A mod uploads a texture once ([`super::Renderer::sky_texture`]) and hands its id with a
//! blend, an alpha and a rotation in each frame's [`super::Frame::sky_layers`]. Textures stay
//! until the mod drops them or the renderer is replaced (a resource reload, a new device).
use glam::{Mat4, Vec3};
use std::sync::atomic::{AtomicU64, Ordering};
use wgpu::util::DeviceExt;

/// How a layer mixes with the sky behind it: OptiFine's `blend` values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkyBlend {
    Alpha,
    #[default]
    Add,
    Subtract,
    Multiply,
    Dodge,
    Burn,
    Screen,
    Overlay,
    Replace,
}

impl SkyBlend {
    pub const ALL: [SkyBlend; 9] = [
        SkyBlend::Alpha,
        SkyBlend::Add,
        SkyBlend::Subtract,
        SkyBlend::Multiply,
        SkyBlend::Dodge,
        SkyBlend::Burn,
        SkyBlend::Screen,
        SkyBlend::Overlay,
        SkyBlend::Replace,
    ];

    fn index(self) -> usize {
        self as usize
    }

    /// OptiFine's blend factors (`Blender.setupBlend`): one function for colour and alpha, as
    /// the Java client's pipelines take them. `None` for Replace (no blending).
    fn state(self) -> Option<wgpu::BlendState> {
        use wgpu::BlendFactor as F;
        let both = |src, dst| {
            let c = wgpu::BlendComponent {
                src_factor: src,
                dst_factor: dst,
                operation: wgpu::BlendOperation::Add,
            };
            Some(wgpu::BlendState { color: c, alpha: c })
        };
        match self {
            SkyBlend::Alpha => Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: F::SrcAlpha,
                    dst_factor: F::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            }),
            SkyBlend::Add => both(F::SrcAlpha, F::One),
            SkyBlend::Subtract => both(F::OneMinusDst, F::Zero),
            SkyBlend::Multiply => both(F::Dst, F::OneMinusSrcAlpha),
            SkyBlend::Dodge => both(F::One, F::One),
            SkyBlend::Burn => both(F::Zero, F::OneMinusSrc),
            SkyBlend::Screen => both(F::One, F::OneMinusSrc),
            SkyBlend::Overlay => both(F::Dst, F::Src),
            SkyBlend::Replace => None,
        }
    }

    /// The colour the texture is multiplied by for a layer at `alpha`: which channels carry
    /// the layer's strength under each blend (OptiFine's `Blender.setupBlend`).
    pub fn color(self, alpha: f32) -> [f32; 4] {
        let a = alpha.clamp(0.0, 1.0);
        match self {
            SkyBlend::Alpha | SkyBlend::Add | SkyBlend::Replace => [1.0, 1.0, 1.0, a],
            SkyBlend::Multiply => [a; 4],
            _ => [a, a, a, 1.0],
        }
    }
}

/// One layer of a mod's custom sky in a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyLayer {
    /// From [`super::Renderer::sky_texture`]; a layer whose texture is gone is skipped.
    pub texture: u64,
    pub blend: SkyBlend,
    /// How strongly it shows, 0 to 1 (the fades, the weather and the mod's brightness).
    pub alpha: f32,
    /// Radians about `axis`, the world's frame (x east, y up, z south).
    pub angle: f32,
    pub axis: [f32; 3],
}

impl SkyLayer {
    /// Camera-relative world from the layer's cube: `angle` about `axis`, none when either
    /// can't be used.
    pub fn rotation(&self) -> Mat4 {
        let axis = Vec3::from_array(self.axis);
        if !self.angle.is_finite() || self.angle == 0.0 || !axis.is_finite() {
            return Mat4::IDENTITY;
        }
        match axis.try_normalize() {
            Some(axis) => Mat4::from_axis_angle(axis, self.angle),
            None => Mat4::IDENTITY,
        }
    }
}

/// The most layers a frame draws; OptiFine's packs have a handful.
pub const MAX_LAYERS: usize = 32;
/// The most textures the renderer keeps for mods at once.
pub const MAX_TEXTURES: usize = 64;
/// Each layer's uniform, at WebGPU's largest offset alignment.
const STRIDE: u64 = 256;

/// Ids no renderer has handed before, so one from a replaced renderer is never another's.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    clip_from_layer: [[f32; 4]; 4],
    color: [f32; 4],
}

struct Texture {
    id: u64,
    group: wgpu::BindGroup,
    // Kept for the bind group's view.
    _texture: wgpu::Texture,
}

pub(super) struct SkyLayers {
    pipelines: Vec<wgpu::RenderPipeline>,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    cube: wgpu::Buffer,
    textures: Vec<Texture>,
    /// This frame's layers' uniforms, `STRIDE` apart (reused: no allocation per frame).
    staged: Vec<u8>,
    /// This frame's draws: the texture's index, the blend and the uniform's offset.
    draws: Vec<(usize, SkyBlend, u32)>,
}

impl SkyLayers {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sky layer bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<Uniform>() as u64
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sky layer shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sky_layers.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sky layer layout"),
            bind_group_layouts: &[Some(&layout)],
            ..Default::default()
        });
        let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];
        let pipelines = SkyBlend::ALL
            .iter()
            .map(|blend| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Sky layer"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_sky_layer"),
                        compilation_options: Default::default(),
                        buffers: &[Some(wgpu::VertexBufferLayout {
                            array_stride: 20,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &attributes,
                        })],
                    },
                    primitive: wgpu::PrimitiveState {
                        cull_mode: None,
                        ..Default::default()
                    },
                    // Behind everything, as the sky is.
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: Some(false),
                        depth_compare: Some(wgpu::CompareFunction::Always),
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_sky_layer"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            blend: blend.state(),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            })
            .collect();
        // Nearest and clamped, as the game samples a texture it loads: no seams between faces.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sky layer sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sky layer uniforms"),
            size: STRIDE * MAX_LAYERS as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cube = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sky layer cube"),
            contents: bytemuck::cast_slice(&cube_vertices()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self {
            pipelines,
            layout,
            sampler,
            uniform,
            cube,
            textures: Vec::new(),
            staged: Vec::with_capacity(STRIDE as usize * MAX_LAYERS),
            draws: Vec::with_capacity(MAX_LAYERS),
        }
    }

    /// Uploads a sky texture (`width` by `height` RGBA8, rows packed): its id, or none when
    /// the size is wrong, too large for the device, or too many are kept.
    pub(super) fn add(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Option<u64> {
        let max = device.limits().max_texture_dimension_2d;
        if width == 0
            || height == 0
            || width > max
            || height > max
            || rgba.len() as u64 != u64::from(width) * u64::from(height) * 4
            || self.textures.len() >= MAX_TEXTURES
        {
            return None;
        }
        // One write, not through the frame's staging belt, which would keep a buffer this
        // large for good.
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("Sky layer texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                // Unorm, as the atlas: the world target holds the game's colours as they are.
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            rgba,
        );
        let view = texture.create_view(&Default::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sky layer resources"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.uniform,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<Uniform>() as u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        self.textures.push(Texture {
            id,
            group,
            _texture: texture,
        });
        Some(id)
    }

    /// Frees texture `id`; nothing if it's not this renderer's.
    pub(super) fn remove(&mut self, id: u64) {
        self.textures.retain(|t| t.id != id);
    }

    pub(super) fn has(&self, id: u64) -> bool {
        self.textures.iter().any(|t| t.id == id)
    }

    /// This frame's layers, `clip_from_world` the camera-relative view projection. Layers past
    /// [`MAX_LAYERS`], without a texture of this renderer's or not shown are left out.
    pub(super) fn prepare(
        &mut self,
        uploads: &mut super::upload::Uploads,
        encoder: &mut wgpu::CommandEncoder,
        clip_from_world: Mat4,
        layers: &[SkyLayer],
    ) {
        self.staged.clear();
        self.draws.clear();
        for layer in layers {
            if self.draws.len() >= MAX_LAYERS {
                break;
            }
            if !layer.alpha.is_finite() || layer.alpha <= 1.0e-4 {
                continue;
            }
            let Some(texture) = self.textures.iter().position(|t| t.id == layer.texture) else {
                continue;
            };
            let uniform = Uniform {
                clip_from_layer: (clip_from_world * layer.rotation()).to_cols_array_2d(),
                color: layer.blend.color(layer.alpha),
            };
            let offset = self.staged.len();
            self.staged.extend_from_slice(bytemuck::bytes_of(&uniform));
            self.staged.resize(offset + STRIDE as usize, 0);
            self.draws.push((texture, layer.blend, offset as u32));
        }
        if !self.staged.is_empty() {
            uploads.buffer(encoder, &self.uniform, 0, &self.staged);
        }
    }

    /// Whether `prepare` left anything to draw.
    pub(super) fn any(&self) -> bool {
        !self.draws.is_empty()
    }

    /// Draws this frame's layers; the caller binds its own group 0 again after.
    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_vertex_buffer(0, self.cube.slice(..));
        for &(texture, blend, offset) in &self.draws {
            pass.set_pipeline(&self.pipelines[blend.index()]);
            pass.set_bind_group(0, &self.textures[texture].group, &[offset]);
            pass.draw(0..36, 0..1);
        }
    }
}

/// The cube 200 across around the camera, six faces of two triangles, each face a sixth of the
/// texture: OptiFine's layout (`CustomSkyLayer.renderSide`), the bottom face from the grid's top
/// left, the top from its top middle. Position then UV.
pub fn cube_vertices() -> Vec<[f32; 5]> {
    use std::f32::consts::{FRAC_PI_2, PI};
    // OptiFine turns every face by Y(-90) X(90) Z(-90) first: the sky's frame, then its own.
    // Together that's X(90); without it the sky lies on its side.
    let frame = Mat4::from_rotation_x(FRAC_PI_2);
    let faces = [
        (Mat4::IDENTITY, 4),
        (Mat4::from_rotation_x(FRAC_PI_2), 1),
        (Mat4::from_rotation_x(-FRAC_PI_2), 0),
        (Mat4::from_rotation_z(FRAC_PI_2), 5),
        (Mat4::from_rotation_z(PI), 2),
        (Mat4::from_rotation_z(PI * 1.5), 3),
    ];
    let mut out = Vec::with_capacity(36);
    for (pose, side) in faces {
        let u0 = (side % 3) as f32 / 3.0;
        let v0 = (side / 3) as f32 * 0.5;
        let (u1, v1) = (u0 + 1.0 / 3.0, v0 + 0.5);
        let corners = [
            ([-100.0, -100.0, -100.0], [u0, v0]),
            ([-100.0, -100.0, 100.0], [u0, v1]),
            ([100.0, -100.0, 100.0], [u1, v1]),
            ([100.0, -100.0, -100.0], [u1, v0]),
        ];
        let vertex = |i: usize| {
            let (p, uv): ([f32; 3], [f32; 2]) = corners[i];
            let p = (frame * pose).transform_point3(Vec3::from_array(p));
            [p.x, p.y, p.z, uv[0], uv[1]]
        };
        out.extend([0, 1, 2, 0, 2, 3].map(vertex));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cube_has_six_faces_a_sixth_of_the_texture_each() {
        let cube = cube_vertices();
        assert_eq!(cube.len(), 36);
        for v in &cube {
            // Every corner on the cube 200 across.
            assert!(
                v[..3].iter().all(|c| (c.abs() - 100.0).abs() < 1.0e-3),
                "{v:?}"
            );
            assert!((0.0..=1.0).contains(&v[3]) && (0.0..=1.0).contains(&v[4]));
        }
        // The bottom face (side 0, third) is the grid's top left, at y = -100.
        let bottom = &cube[12..18];
        assert!(bottom.iter().all(|v| (v[1] + 100.0).abs() < 1.0e-3));
        assert!(bottom.iter().all(|v| v[3] <= 1.0 / 3.0 + 1.0e-4 && v[4] <= 0.5));
        // The top face (side 1, second) is the grid's top middle, at y = 100.
        let top = &cube[6..12];
        assert!(top.iter().all(|v| (v[1] - 100.0).abs() < 1.0e-3));
        assert!(
            top.iter()
                .all(|v| (1.0 / 3.0 - 1.0e-4..=2.0 / 3.0 + 1.0e-4).contains(&v[3]) && v[4] <= 0.5)
        );
        // The rest stand around the horizon.
        for face in [&cube[..6], &cube[18..24], &cube[24..30], &cube[30..36]] {
            assert!(face.iter().all(|v| v[1].abs() < 100.0 + 1.0e-3));
            assert!(face.iter().any(|v| (v[1] + 100.0).abs() < 1.0e-3));
            assert!(face.iter().any(|v| (v[1] - 100.0).abs() < 1.0e-3));
        }
    }

    #[test]
    fn blends_carry_the_alpha_as_optifine_does() {
        assert_eq!(SkyBlend::Add.color(0.5), [1.0, 1.0, 1.0, 0.5]);
        assert_eq!(SkyBlend::Multiply.color(0.5), [0.5; 4]);
        assert_eq!(SkyBlend::Screen.color(0.5), [0.5, 0.5, 0.5, 1.0]);
        assert_eq!(SkyBlend::Replace.color(2.0), [1.0, 1.0, 1.0, 1.0]);
        assert!(SkyBlend::Replace.state().is_none());
        assert_eq!(
            SkyBlend::ALL.map(SkyBlend::index),
            [0, 1, 2, 3, 4, 5, 6, 7, 8]
        );
    }

    #[test]
    fn a_layer_turns_about_its_axis() {
        let layer = SkyLayer {
            texture: 1,
            blend: SkyBlend::Add,
            alpha: 1.0,
            angle: FRAC_PI_2_F,
            axis: [0.0, 0.0, 2.0],
        };
        let up = layer.rotation().transform_vector3(Vec3::Y);
        assert!((up - Vec3::new(-1.0, 0.0, 0.0)).length() < 1.0e-5, "{up}");
        let still = SkyLayer {
            axis: [0.0; 3],
            ..layer
        };
        assert_eq!(still.rotation(), Mat4::IDENTITY);
        let nan = SkyLayer {
            angle: f32::NAN,
            ..layer
        };
        assert_eq!(nan.rotation(), Mat4::IDENTITY);
    }

    const FRAC_PI_2_F: f32 = std::f32::consts::FRAC_PI_2;
}
