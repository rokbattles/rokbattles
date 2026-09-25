//! GPU resources and triangle generation shared by WebGPU and WebGL2.
//!
//! A frame draws coarse-to-fine tiles, uploaded map images, forbidden terrain,
//! overlays/previews, solid annotations, then atlas text. Geometry uses one sampled
//! texture pipeline with straight-alpha blending. Surfaces that apply sRGB encoding
//! receive a separate presentation pass after display-space compositing.
//!
//! Tile textures have a bounded residency cache. Full-map images and named sprites
//! are owned separately; replacing a texture or dropping the renderer releases it.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bytemuck::{Pod, Zeroable};
use serde::Deserialize;
use web_sys::HtmlCanvasElement;

#[derive(Debug)]
struct WebDisplay;

impl wgpu::rwh::HasDisplayHandle for WebDisplay {
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
        Ok(wgpu::rwh::DisplayHandle::web())
    }
}

use crate::scene::{Object, Scene, Shape, Style, Tile, shape_points};

/// Maximum resident pyramid tiles, independent of map images, sprites and the glyph atlas.
pub const CACHE_LIMIT: usize = 80;

// Keep this layout synchronized with the vertex attributes and map.wgsl.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    stroke: [f32; 4],
}

struct Texture {
    texture: wgpu::Texture,
    bind: wgpu::BindGroup,
    // Last draw/upload tick, used to choose victims outside the needed tile set.
    used: u64,
}

// WebGL commonly exposes an sRGB surface, while browser WebGPU uses unorm.
// Composite UI colors into unorm first so translucent overlays blend identically.
// Only the final opaque copy converts into the surface's linear input space.
struct Presentation {
    pipeline: wgpu::RenderPipeline,
    target: Texture,
}

impl Drop for Texture {
    fn drop(&mut self) {
        self.texture.destroy()
    }
}

/// A catalog sprite or territory rectangle. Sizes may use CSS pixels or world units.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Overlay {
    /// World-space center of the image or rectangle.
    pub position: [f32; 2],
    /// Full width and height, interpreted according to `world_size`.
    /// With a stroke, one zero dimension draws a single boundary segment.
    pub size: [f32; 2],
    /// Uploaded image key, or `"white"` for solid fills and outlines.
    pub texture: String,
    /// Straight-alpha RGBA tint in display space.
    pub color: [f32; 4],
    /// Uses native map units for size when true, CSS pixels otherwise.
    pub world_size: bool,
    /// CSS-pixel offset from the projected center; positive Y moves downward.
    #[serde(default)]
    pub offset: [f32; 2],
    /// Multiplies opacity by the overview-to-detail fade.
    #[serde(default)]
    pub detail_only: bool,
    /// Fades in over one magnification unit above this threshold; zero disables it.
    #[serde(default)]
    pub min_zoom: f32,
    /// CSS-pixel outline width; zero draws a filled textured quad.
    #[serde(default)]
    pub stroke_width: f32,
    /// Applies shader dashes to an outline; ignored for filled quads.
    #[serde(default)]
    pub dashed: bool,
}

/// Owns one canvas surface, its device, uploaded textures and reusable geometry buffer.
pub struct Gpu {
    pub backend: &'static str,
    raster: [Option<Texture>; 2],
    sprites: HashMap<String, Texture>,
    pub overlays: Vec<Overlay>,
    pub preview: Vec<Overlay>,
    pub(crate) forbidden: Vec<[[f32; 2]; 3]>,
    // Retain the backend handles for the lifetime of the canvas surface.
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    presentation: Option<Presentation>,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    buffer: wgpu::Buffer,
    capacity: u64,
    tiles: HashMap<Tile, Texture>,
    white: Texture,
    font: Texture,
    clock: u64,
    pub uploaded: u32,
    pub drawn: u32,
    pub frames: u32,
    fault: Arc<Mutex<Option<String>>>,
}

impl Gpu {
    /// Creates resources for one backend; the caller owns fallback and frame scheduling.
    ///
    /// Uses WebGL2-compatible limits for both backends so geometry follows the same
    /// path. Adapter texture limits are retained for full-resolution map uploads.
    pub async fn with_backend(
        canvas: HtmlCanvasElement,
        atlas: Vec<u8>,
        webgl: bool,
    ) -> Result<Self, String> {
        if atlas.len() != 384 * 240 * 4 {
            return Err("Invalid glyph atlas".into());
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: if webgl { wgpu::Backends::GL } else { wgpu::Backends::BROWSER_WEBGPU },
            display: Some(Box::new(WebDisplay)),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| e.to_string())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::LowPower,
                ..Default::default()
            })
            .await
            .map_err(|e| format!("WebGPU adapter unavailable: {e}"))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ROK Battles map engine"),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        // Device callbacks run outside render(); retain failures for the next frame.
        let fault = Arc::new(Mutex::new(None));
        let errors = Arc::clone(&fault);
        device.on_uncaptured_error(Arc::new(move |error| {
            if let Ok(mut value) = errors.lock() {
                *value = Some(error.to_string())
            }
        }));
        let lost = Arc::clone(&fault);
        device.set_device_lost_callback(move |reason, message| {
            if reason != wgpu::DeviceLostReason::Destroyed
                && let Ok(mut value) = lost.lock()
            {
                *value = Some(format!("GPU device lost: {message}"))
            }
        });
        let config =
            surface.get_default_config(&adapter, 1, 1).ok_or("No WebGPU surface configuration")?;
        surface.configure(&device, &config);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Map tiles and annotations"),
            source: wgpu::ShaderSource::Wgsl(include_str!("map.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Map renderer"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4],
                }],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: if config.format.is_srgb() {
                        wgpu::TextureFormat::Rgba8Unorm
                    } else {
                        config.format
                    },
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let layout = pipeline.get_bind_group_layout(0);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Bilinear tile sampling"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let presentation = config.format.is_srgb().then(|| {
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Map sRGB presentation"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_present"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_present"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
            let target = Self::texture(&device, &pipeline.get_bind_group_layout(0), &sampler, 1, 1);
            Presentation { pipeline, target }
        });
        // A white texel lets solid shapes use the same sampled-texture pipeline.
        let white = Self::texture(&device, &layout, &sampler, 1, 1);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &white.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255; 4],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );
        // The controller packs printable ASCII into 16 columns of 24×40 cells.
        let font = Self::texture(&device, &layout, &sampler, 384, 240);
        Self::copy_pixels(&queue, &font.texture, &atlas);
        let capacity = 65536;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reusable geometry"),
            size: capacity,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Self {
            backend: if adapter.get_info().backend == wgpu::Backend::Gl {
                "WebGL2"
            } else {
                "WebGPU"
            },
            raster: [None, None],
            sprites: HashMap::new(),
            overlays: Vec::new(),
            preview: Vec::new(),
            forbidden: Vec::new(),
            _instance: instance,
            _adapter: adapter,
            surface,
            device,
            queue,
            config,
            pipeline,
            presentation,
            layout,
            sampler,
            buffer,
            capacity,
            tiles: HashMap::new(),
            white,
            font,
            clock: 0,
            uploaded: 0,
            drawn: 0,
            frames: 0,
            fault,
        })
    }

    fn texture(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        w: u32,
        h: u32,
    ) -> Texture {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Map image"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Map image binding"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        Texture { texture, bind, used: 0 }
    }

    // RGBA uploads avoid browser-specific failures seen with external image copies.
    fn copy_pixels(queue: &wgpu::Queue, texture: &wgpu::Texture, pixels: &[u8]) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture.width() * 4),
                rows_per_image: Some(texture.height()),
            },
            wgpu::Extent3d {
                width: texture.width(),
                height: texture.height(),
                depth_or_array_layers: 1,
            },
        );
    }

    /// Copies RGBA8 pixels into a full-map slot or named sprite, replacing its old texture.
    pub fn upload_image(
        &mut self,
        key: &str,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<(), String> {
        let limit = self.device.limits().max_texture_dimension_2d.min(8192);
        if width == 0
            || height == 0
            || width > limit
            || height > limit
            || pixels.len() as u64 != u64::from(width) * u64::from(height) * 4
        {
            return Err("Invalid image dimensions".into());
        }
        if key.len() > 128 || (!self.sprites.contains_key(key) && self.sprites.len() >= 256) {
            return Err("Sprite cache limit reached".into());
        }
        let texture = Self::texture(&self.device, &self.layout, &self.sampler, width, height);
        Self::copy_pixels(&self.queue, &texture.texture, pixels);
        match key {
            "overview" => self.raster[0] = Some(texture),
            "detail" => self.raster[1] = Some(texture),
            _ => {
                self.sprites.insert(key.to_owned(), texture);
            }
        }
        Ok(())
    }

    /// Drops generated label textures that are no longer referenced by the replacement overlay set.
    pub fn retain_label_images<'a>(&mut self, retained: impl Iterator<Item = &'a str>) {
        let retained: std::collections::HashSet<_> = retained.collect();
        self.sprites.retain(|key, _| !key.starts_with("label-") || retained.contains(key.as_str()));
    }

    pub fn has(&self, t: Tile) -> bool {
        self.tiles.contains_key(&t)
    }

    pub fn count(&self) -> usize {
        self.tiles.len()
    }

    /// Returns resident tile pixel bytes, excluding all other GPU allocations.
    pub fn bytes(&self) -> u64 {
        self.tiles
            .values()
            .map(|t| u64::from(t.texture.width()) * u64::from(t.texture.height()) * 4)
            .sum()
    }

    /// Adds a needed tile, evicting only tiles outside the current fallback/detail set.
    ///
    /// Validation precedes stale-response checks, so malformed uploads still fail
    /// when their camera request is no longer current.
    pub fn upload(&mut self, t: Tile, pixels: &[u8], needed: &[Tile]) -> Result<(), String> {
        if !t.valid() {
            return Err("Invalid tile key".into());
        }
        let [width, height] = t.dimensions();
        if pixels.len() != (width * height * 4) as usize {
            return Err("Unexpected tile dimensions".into());
        }
        if !needed.contains(&t) || self.has(t) {
            return Ok(());
        }
        // Evict the least recently drawn tile outside the current fallback/detail set.
        while self.tiles.len() >= CACHE_LIMIT {
            let victim = self
                .tiles
                .iter()
                .filter(|(key, _)| !needed.contains(key))
                .min_by_key(|(_, value)| value.used)
                .map(|(key, _)| *key);
            if let Some(key) = victim {
                self.tiles.remove(&key);
            } else {
                return Err("Visible tiles exceed the cache budget".into());
            }
        }
        let mut texture = Self::texture(&self.device, &self.layout, &self.sampler, width, height);
        Self::copy_pixels(&self.queue, &texture.texture, pixels);
        texture.used = self.clock;
        self.tiles.insert(t, texture);
        self.uploaded += 1;
        Ok(())
    }

    /// Resizes the backing surface in physical pixels, clamped to the device limit.
    pub fn resize(&mut self, w: u32, h: u32) {
        let limit = self.device.limits().max_texture_dimension_2d;
        let w = w.clamp(1, limit);
        let h = h.clamp(1, limit);
        if self.config.width != w || self.config.height != h {
            self.config.width = w;
            self.config.height = h;
            self.surface.configure(&self.device, &self.config);
            if let Some(presentation) = &mut self.presentation {
                presentation.target = Self::texture(
                    &self.device,
                    &presentation.pipeline.get_bind_group_layout(0),
                    &self.sampler,
                    w,
                    h,
                );
            }
        }
    }

    /// Submits an interactive frame, or skips a temporarily unavailable surface.
    pub fn render(&mut self, scene: &Scene) -> Result<(), String> {
        self.render_frame(scene, false)
    }

    /// Draws the detailed map and every supplied overlay without interactive zoom fades.
    pub fn render_export(&mut self, scene: &Scene) -> Result<(), String> {
        self.render_frame(scene, true)
    }

    fn render_frame(&mut self, scene: &Scene, exporting: bool) -> Result<(), String> {
        if let Ok(fault) = self.fault.lock()
            && let Some(message) = fault.as_ref()
        {
            return Err(message.clone());
        }
        self.clock += 1;
        let mut vertices = Vec::new();
        let mut batches = Vec::new();
        let mut images = Vec::new();
        let zoom = scene.scale / scene.fit_scale();
        let detail_opacity = if exporting { 1.0 } else { (zoom - 1.0).clamp(0.0, 1.0) };
        let [west, south, east, north] = scene.config.bounds;
        let a = scene.world_to_screen([west, north]);
        let b = scene.world_to_screen([east, south]);
        for (index, key) in ["overview", "detail"].iter().enumerate() {
            if self.raster.get(index).is_some_and(Option::is_some) {
                let start = vertices.len() as u32;
                let alpha = if index == 0 || self.raster.first().is_none_or(Option::is_none) {
                    1.0
                } else {
                    detail_opacity
                };
                quad(
                    &mut vertices,
                    scene,
                    [a[0], a[1], b[0], b[1]],
                    [0.0, 0.0, 1.0, 1.0],
                    [1.0, 1.0, 1.0, alpha],
                );
                images.push((*key, start..vertices.len() as u32));
            }
        }
        // Share the detail image's fade, keeping forbidden terrain absent from
        // the overview. The shader's negative UV sentinel selects screen-space
        // hatching without adding a texture or changing sprite sampling.
        if detail_opacity > 0.0 && self.raster[1].is_some() {
            let start = vertices.len() as u32;
            for triangle in &self.forbidden {
                let points = triangle.map(|point| scene.world_to_screen(point));
                if points.iter().all(|p| p[0] < 0.0)
                    || points.iter().all(|p| p[1] < 0.0)
                    || points.iter().all(|p| p[0] > scene.width)
                    || points.iter().all(|p| p[1] > scene.height)
                {
                    continue;
                }
                for [x, y] in points {
                    vertices.push(Vertex {
                        position: [x / scene.width * 2.0 - 1.0, 1.0 - y / scene.height * 2.0],
                        uv: [-1.0, -1.0],
                        color: [1.0, 0.12, 0.06, 0.24 * detail_opacity],
                        stroke: [0.0; 4],
                    });
                }
            }
            images.push(("white", start..vertices.len() as u32));
        }
        // needed_tiles() is coarse-first, so missing detail leaves its parent visible.
        for t in scene.needed_tiles() {
            if let Some(texture) = self.tiles.get_mut(&t) {
                texture.used = self.clock;
                let start = vertices.len() as u32;
                tile_quad(&mut vertices, scene, t);
                batches.push((t, start..vertices.len() as u32));
            }
        }
        self.drawn = batches.len() as u32;
        for overlay in self.overlays.iter().chain(&self.preview) {
            let mut opacity = if overlay.detail_only { detail_opacity } else { 1.0 };
            if !exporting && overlay.min_zoom > 0.0 {
                opacity *= (zoom - overlay.min_zoom).clamp(0.0, 1.0);
            }
            if opacity <= 0.0 {
                continue;
            }
            let [x, y] = scene.world_to_screen(overlay.position);
            let center = [x + overlay.offset[0], y + overlay.offset[1]];
            let scale = if overlay.world_size { scene.scale } else { 1.0 };
            let [w, h] = overlay.size.map(|n| n * scale / 2.0);
            if center[0] + w < 0.0
                || center[1] + h < 0.0
                || center[0] - w > scene.width
                || center[1] - h > scene.height
            {
                continue;
            }
            let start = vertices.len() as u32;
            let mut color = overlay.color;
            color[3] *= opacity;
            let rect = [center[0] - w, center[1] - h, center[0] + w, center[1] + h];
            if overlay.stroke_width > 0.0 {
                outline(&mut vertices, scene, rect, overlay.stroke_width, overlay.dashed, color);
            } else {
                quad(&mut vertices, scene, rect, [0.0, 0.0, 1.0, 1.0], color);
            }
            images.push((overlay.texture.as_str(), start..vertices.len() as u32));
        }
        let solid_start = vertices.len() as u32;
        for o in &scene.objects {
            let selected = scene.selected.contains(&o.id());
            let color = o.style().rgba();
            let factor = o.style().factor();
            match o {
                Object::Marker { x, y, .. } => {
                    let p = scene.world_to_screen([*x, *y]);
                    circle(
                        &mut vertices,
                        scene,
                        p,
                        12.0 * factor,
                        if selected { [1.0; 4] } else { [0.03, 0.08, 0.13, 0.95] },
                    );
                    circle(&mut vertices, scene, p, 9.0 * factor, color);
                    circle(&mut vertices, scene, p, 3.0, [1.0; 4]);
                }
                Object::Drawing { points, shape, style, .. } => {
                    if selected {
                        draw_shape(&mut vertices, scene, points, *shape, *style, [1.0; 4], 3.0);
                    }
                    draw_shape(&mut vertices, scene, points, *shape, *style, color, 0.0);
                }
                Object::Text { x, y, text, .. } => {
                    let p = scene.world_to_screen([*x, *y]);
                    quad(
                        &mut vertices,
                        scene,
                        [
                            p[0] - 4.0,
                            p[1] - 13.0 * factor,
                            p[0] + 12.0 * factor * text.len() as f32 + 4.0,
                            p[1] + 13.0 * factor,
                        ],
                        [0.0, 0.0, 1.0, 1.0],
                        if selected { [0.35, 0.23, 0.05, 0.95] } else { [0.025, 0.05, 0.09, 0.9] },
                    );
                }
            }
        }
        draw_shape(
            &mut vertices,
            scene,
            &scene.draft,
            scene.draft_shape,
            scene.brush,
            scene.brush.rgba(),
            0.0,
        );
        // Draw glyphs last so label backgrounds cannot cover neighboring text.
        let font_start = vertices.len() as u32;
        for o in &scene.objects {
            if let Object::Text { x, y, text, .. } = o {
                let p = scene.world_to_screen([*x, *y]);
                let factor = o.style().factor();
                for (i, c) in text.bytes().enumerate() {
                    let index = u32::from(c) - 32;
                    let col = (index % 16) as f32;
                    let row = (index / 16) as f32;
                    quad(
                        &mut vertices,
                        scene,
                        [
                            p[0] + i as f32 * 12.0 * factor,
                            p[1] - 10.0 * factor,
                            p[0] + i as f32 * 12.0 * factor + 12.0 * factor,
                            p[1] + 10.0 * factor,
                        ],
                        [col / 16.0, row / 6.0, (col + 1.0) / 16.0, (row + 1.0) / 6.0],
                        o.style().rgba(),
                    );
                }
            }
        }
        let bytes = bytemuck::cast_slice(&vertices);
        // Grow geometrically and reuse the buffer on subsequent frames.
        if bytes.len() as u64 > self.capacity {
            self.capacity = (bytes.len() as u64).next_power_of_two();
            self.buffer.destroy();
            self.buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Reusable geometry"),
                size: self.capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !bytes.is_empty() {
            self.queue.write_buffer(&self.buffer, 0, bytes)
        }
        // Do not treat a hidden canvas or a resize race as device loss. The host
        // can request another frame; a permanently lost device must be recreated.
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            _ => return Err("WebGPU surface lost; reload to recreate the renderer".into()),
        };
        let view = self
            .presentation
            .as_ref()
            .map_or(&frame.texture, |presentation| &presentation.target.texture)
            .create_view(&Default::default());
        let background = wgpu::Color { r: 39.0 / 255.0, g: 39.0 / 255.0, b: 42.0 / 255.0, a: 1.0 };
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Map frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(background),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.buffer.slice(..));
            for (key, range) in batches {
                if let Some(t) = self.tiles.get(&key) {
                    pass.set_bind_group(0, &t.bind, &[]);
                    pass.draw(range, 0..1)
                }
            }
            for (key, range) in images {
                let texture = match key {
                    "overview" => self.raster[0].as_ref(),
                    "detail" => self.raster[1].as_ref(),
                    "white" => Some(&self.white),
                    _ => self.sprites.get(key),
                };
                if let Some(texture) = texture {
                    pass.set_bind_group(0, &texture.bind, &[]);
                    pass.draw(range, 0..1);
                }
            }
            if font_start > solid_start {
                pass.set_bind_group(0, &self.white.bind, &[]);
                pass.draw(solid_start..font_start, 0..1)
            }
            if vertices.len() as u32 > font_start {
                pass.set_bind_group(0, &self.font.bind, &[]);
                pass.draw(font_start..vertices.len() as u32, 0..1)
            }
        }
        if let Some(presentation) = &self.presentation {
            let surface_view = frame.texture.create_view(&Default::default());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Map presentation"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&presentation.pipeline);
            pass.set_bind_group(0, &presentation.target.bind, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        frame.present();
        self.frames += 1;
        Ok(())
    }
}

impl Drop for Gpu {
    fn drop(&mut self) {
        self.buffer.destroy();
        self.device.destroy();
    }
}

fn quad(vertices: &mut Vec<Vertex>, scene: &Scene, rect: [f32; 4], uv: [f32; 4], color: [f32; 4]) {
    // All CPU tessellation uses CSS pixels. Normalization here lets the viewport
    // apply device pixel ratio once, without scaling annotation widths twice.
    let [x0, y0, x1, y1] = rect;
    let [u0, v0, u1, v1] = uv;
    for (x, y, u, w) in [
        (x0, y0, u0, v0),
        (x0, y1, u0, v1),
        (x1, y0, u1, v0),
        (x1, y0, u1, v0),
        (x0, y1, u0, v1),
        (x1, y1, u1, v1),
    ] {
        vertices.push(Vertex {
            position: [x / scene.width * 2.0 - 1.0, 1.0 - y / scene.height * 2.0],
            uv: [u, w],
            color,
            stroke: [0.0; 4],
        })
    }
}

fn outline(
    vertices: &mut Vec<Vertex>,
    scene: &Scene,
    [x0, y0, x1, y1]: [f32; 4],
    width: f32,
    dashed: bool,
    color: [f32; 4],
) {
    // A zero-height/width rectangle is a single boundary segment, not two
    // coincident edges. This also supports union outlines supplied by the host.
    if x1 > x0 {
        stroke(vertices, scene, [x0, y0, x1, y0], width, dashed, color);
        if y1 > y0 {
            stroke(vertices, scene, [x0, y1, x1, y1], width, dashed, color);
        }
    }
    if y1 > y0 {
        stroke(vertices, scene, [x0, y0, x0, y1], width, dashed, color);
        if x1 > x0 {
            stroke(vertices, scene, [x1, y0, x1, y1], width, dashed, color);
        }
    }
}

fn stroke(
    vertices: &mut Vec<Vertex>,
    scene: &Scene,
    [x0, y0, x1, y1]: [f32; 4],
    width: f32,
    dashed: bool,
    color: [f32; 4],
) {
    let half = width / 2.0;
    // Include the antialiasing fringe. The fragment shader integrates coverage
    // across the edge instead of rounding a fractional width to one or two pixels.
    let extent = half + 1.0 / scene.dpr;
    let horizontal = x1 > x0;
    let rect = if horizontal {
        [x0, y0 - extent, x1, y1 + extent]
    } else {
        [x0 - extent, y0, x1 + extent, y1]
    };
    let start = vertices.len();
    quad(vertices, scene, rect, [0.0, 0.0, 1.0, 1.0], color);
    for vertex in vertices.iter_mut().skip(start) {
        let [u, v] = vertex.uv;
        let (along, across) = if horizontal {
            (x0 + u * (x1 - x0), (v * 2.0 - 1.0) * extent)
        } else {
            (y0 + v * (y1 - y0), (u * 2.0 - 1.0) * extent)
        };
        vertex.stroke = [along, across, half, if dashed { 1.0 } else { 0.0 }];
    }
}

fn tile_quad(vertices: &mut Vec<Vertex>, scene: &Scene, tile: Tile) {
    let size = (512u32 << tile.level) as f32;
    let [x0, y0, x1, y1] = scene.config.bounds;
    let px = tile.x as f32 * 512.0;
    let py = tile.y as f32 * 512.0;
    let a = scene.world_to_screen([x0 + px / size * (x1 - x0), y1 - py / size * (y1 - y0)]);
    let b = scene.world_to_screen([
        x0 + (px + 512.0) / size * (x1 - x0),
        y1 - (py + 512.0) / size * (y1 - y0),
    ]);
    let [w, h] = tile.dimensions();
    // Sample the core, retaining the surrounding texels for seam-free filtering.
    let u = if tile.x > 0 { 1.0 } else { 0.0 };
    let z = if tile.y > 0 { 1.0 } else { 0.0 };
    quad(
        vertices,
        scene,
        [a[0], a[1], b[0], b[1]],
        [u / w as f32, z / h as f32, (u + 512.0) / w as f32, (z + 512.0) / h as f32],
        [1.0; 4],
    );
}

fn triangle(vertices: &mut Vec<Vertex>, scene: &Scene, points: [[f32; 2]; 3], color: [f32; 4]) {
    for p in points {
        vertices.push(Vertex {
            position: [p[0] / scene.width * 2.0 - 1.0, 1.0 - p[1] / scene.height * 2.0],
            uv: [0.5; 2],
            color,
            stroke: [0.0; 4],
        })
    }
}

fn circle(vertices: &mut Vec<Vertex>, scene: &Scene, p: [f32; 2], radius: f32, color: [f32; 4]) {
    for i in 0..24 {
        let a = i as f32 * std::f32::consts::TAU / 24.0;
        let b = (i + 1) as f32 * std::f32::consts::TAU / 24.0;
        triangle(
            vertices,
            scene,
            [
                p,
                [p[0] + a.cos() * radius, p[1] + a.sin() * radius],
                [p[0] + b.cos() * radius, p[1] + b.sin() * radius],
            ],
            color,
        )
    }
}

fn draw_line(
    vertices: &mut Vec<Vertex>,
    scene: &Scene,
    points: &[[f32; 2]],
    width: f32,
    color: [f32; 4],
) {
    for pair in points.array_windows::<2>() {
        let a = scene.world_to_screen(pair[0]);
        let b = scene.world_to_screen(pair[1]);
        let d = [b[0] - a[0], b[1] - a[1]];
        let length = (d[0] * d[0] + d[1] * d[1]).sqrt().max(0.001);
        let n = [-d[1] / length * width / 2.0, d[0] / length * width / 2.0];
        let p = [a[0] + n[0], a[1] + n[1]];
        let q = [a[0] - n[0], a[1] - n[1]];
        let r = [b[0] + n[0], b[1] + n[1]];
        let t = [b[0] - n[0], b[1] - n[1]];
        triangle(vertices, scene, [p, q, r], color);
        triangle(vertices, scene, [r, q, t], color);
    }
}

fn draw_shape(
    vertices: &mut Vec<Vertex>,
    scene: &Scene,
    points: &[[f32; 2]],
    shape: Shape,
    style: Style,
    color: [f32; 4],
    extra: f32,
) {
    if shape != Shape::Arrow {
        draw_line(
            vertices,
            scene,
            &shape_points(points, shape),
            style.size as f32 * 2.0 + extra,
            color,
        );
        return;
    }
    let [start, tip] = points else {
        return;
    };
    let a = scene.world_to_screen(*start);
    let b = scene.world_to_screen(*tip);
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length = (dx * dx + dy * dy).sqrt();
    if length < 3.0 {
        return;
    }
    let n = [dx / length, dy / length];
    let half = style.size as f32;
    let head = (10.0 + style.size as f32 * 4.0).min(length * 0.5);
    let at = |along: f32, across: f32| {
        [a[0] + n[0] * along - n[1] * across, a[1] + n[1] * along + n[0] * across]
    };
    let outline = [
        at(0.0, half),
        at(length - head, half),
        at(length - head, head * 0.5),
        b,
        at(length - head, -head * 0.5),
        at(length - head, -half),
        at(0.0, -half),
        at(0.0, half),
    ];
    if extra > 0.0 {
        let world: Vec<_> = outline.iter().map(|p| scene.screen_to_world(p[0], p[1])).collect();
        draw_line(vertices, scene, &world, extra, color);
        return;
    }
    // The shaft ends at the head base; no stroke can protrude through the tip.
    triangle(vertices, scene, [outline[0], outline[6], outline[1]], color);
    triangle(vertices, scene, [outline[1], outline[6], outline[5]], color);
    triangle(vertices, scene, [outline[2], outline[3], outline[4]], color);
}
