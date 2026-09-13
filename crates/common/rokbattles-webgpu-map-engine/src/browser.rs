//! WASM commands used by the frontend controller.
//!
//! The controller owns events, image decoding and frame scheduling. This adapter
//! keeps scene objects and GPU resources in Rust, returning camera status and tile
//! requests to JavaScript. The host supplies the current annotations as scene JSON.

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::{
    Config, Scene,
    gpu::{CACHE_LIMIT, Gpu},
    scene::Tile,
};

fn error(message: impl ToString) -> JsValue {
    JsValue::from_str(&message.to_string())
}

fn decode_overlays(json: &str, maximum: usize) -> Result<Vec<crate::gpu::Overlay>, JsValue> {
    if json.len() > 4 * 1024 * 1024 {
        return Err(error("Overlay payload too large"));
    }
    let overlays: Vec<crate::gpu::Overlay> = serde_json::from_str(json).map_err(error)?;
    if overlays.len() > maximum
        || overlays.iter().any(|overlay| {
            !overlay
                .position
                .iter()
                .chain(overlay.size.iter())
                .chain(overlay.color.iter())
                .chain(overlay.offset.iter())
                .all(|number| number.is_finite())
                || overlay.size.iter().any(|number| *number < 0.0 || *number > 20_000.0)
                || overlay.offset.iter().any(|number| number.abs() > 4_096.0)
                || !overlay.stroke_width.is_finite()
                || !(0.0..=8.0).contains(&overlay.stroke_width)
                || !overlay.min_zoom.is_finite()
                || !(0.0..=32.0).contains(&overlay.min_zoom)
                || overlay.texture.len() > 128
        })
    {
        return Err(error("Invalid overlays"));
    }
    Ok(overlays)
}

/// Browser-facing command interface. Decoded objects are retained in Rust.
#[wasm_bindgen]
pub struct MapEngine {
    scene: Scene,
    gpu: Gpu,
}

#[wasm_bindgen]
impl MapEngine {
    /// Creates an empty map on WebGPU, or WebGL2 when `webgl` is true.
    ///
    /// `config` is a JSON [`Config`]. `atlas` contains 384×240 tightly packed RGBA8
    /// pixels for printable ASCII in 16 columns of 24×40 cells. Map images and
    /// overlays are uploaded separately; creation does not draw a frame.
    ///
    /// Backend fallback is a host decision. A canvas already used by one backend
    /// must be replaced before attempting the other.
    ///
    /// # Errors
    ///
    /// Rejects invalid configuration or atlas dimensions, or failure to obtain
    /// an adapter, device or canvas surface for the requested backend.
    pub async fn create_raster(
        canvas: HtmlCanvasElement,
        config: &str,
        atlas: Vec<u8>,
        webgl: bool,
    ) -> Result<MapEngine, JsValue> {
        let config: Config = serde_json::from_str(config).map_err(error)?;
        let scene = Scene::empty(config).map_err(error)?;
        let gpu = Gpu::with_backend(canvas, atlas, webgl).await.map_err(error)?;
        Ok(Self { scene, gpu })
    }

    /// Uploads tightly packed, unpremultiplied RGBA8 pixels under a texture key.
    ///
    /// `"overview"` and `"detail"` replace the two full-map images. Other keys name
    /// sprites or labels referenced by overlays. Pixel bytes are copied during
    /// the call, so the host can release its decoded image afterward.
    ///
    /// # Errors
    ///
    /// Rejects mismatched byte lengths, zero or unsupported dimensions, keys longer
    /// than 128 bytes, or additions that exceed the sprite cache limit.
    pub fn upload_image(
        &mut self,
        key: &str,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<(), JsValue> {
        self.gpu.upload_image(key, width, height, pixels).map_err(error)
    }

    /// Replaces the sprite and rectangle overlays after validating the whole array.
    ///
    /// Unreferenced `label-` textures are released on success. Upload new labels
    /// before referencing them; overlays with missing textures are skipped at draw time.
    ///
    /// # Errors
    ///
    /// Invalid JSON, non-finite geometry, invalid size/style ranges, more than
    /// 20,000 overlays or a payload larger than 4 MiB leave existing overlays intact.
    pub fn set_overlays(&mut self, json: &str) -> Result<(), JsValue> {
        let overlays = decode_overlays(json, 20_000)?;
        self.gpu.retain_label_images(overlays.iter().map(|overlay| overlay.texture.as_str()));
        self.gpu.overlays = overlays;
        Ok(())
    }

    /// Replaces transient cursor overlays without adding them to the scene document.
    ///
    /// # Errors
    ///
    /// Uses the same validation as [`Self::set_overlays`], with a limit of 16 entries.
    /// Failure preserves the previous preview.
    pub fn set_preview(&mut self, json: &str) -> Result<(), JsValue> {
        self.gpu.preview = decode_overlays(json, 16)?;
        Ok(())
    }

    /// Converts canvas CSS coordinates to the native map coordinate system.
    pub fn screen_to_world(&self, x: f32, y: f32) -> Vec<f32> {
        self.scene.screen_to_world(x, y).to_vec()
    }

    /// Converts native map coordinates to canvas CSS coordinates.
    pub fn world_to_screen(&self, x: f32, y: f32) -> Vec<f32> {
        self.scene.world_to_screen([x, y]).to_vec()
    }

    #[expect(
        clippy::cast_sign_loss,
        reason = "Scene::resize clamps dimensions and DPR to positive values"
    )]
    /// Resizes the viewport using CSS dimensions and device pixel ratio.
    pub fn resize(&mut self, width: f32, height: f32, dpr: f32) {
        self.scene.resize(width, height, dpr);
        self.gpu.resize(
            (self.scene.width * self.scene.dpr) as u32,
            (self.scene.height * self.scene.dpr) as u32,
        )
    }

    /// Pans by a CSS pixel delta.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.scene.pan(dx, dy)
    }

    /// Zooms around a canvas-relative CSS pixel position.
    pub fn zoom(&mut self, factor: f32, x: f32, y: f32) {
        self.scene.zoom(factor, x, y)
    }

    /// Fits the full map in the viewport.
    pub fn fit(&mut self) {
        self.scene.fit()
    }

    /// Picks an annotation, optionally toggling its selection membership.
    pub fn select(&mut self, x: f32, y: f32, additive: bool) {
        self.scene.select(x, y, additive)
    }

    /// Updates the brush and selection with color ID 1–8 and size 1–3.
    ///
    /// # Errors
    ///
    /// Invalid preset IDs leave the current styles unchanged.
    pub fn set_style(&mut self, color: u8, size: u8) -> Result<(), JsValue> {
        self.scene.set_style(color, size).map_err(error)
    }

    /// Starts a draft at a CSS pixel position using `"drawing"`, `"arrow"` or `"circle"`.
    ///
    /// # Errors
    ///
    /// An unknown kind leaves the previous draft unchanged. See [`Scene::begin_drawing`].
    pub fn begin_drawing(&mut self, kind: &str, x: f32, y: f32) -> Result<(), JsValue> {
        self.scene.begin_drawing(kind, x, y).map_err(error)
    }

    /// Updates the active draft from a CSS pixel position.
    pub fn draw_point(&mut self, x: f32, y: f32) {
        self.scene.draw_point(x, y)
    }

    /// Discards the active draft.
    pub fn cancel_drawing(&mut self) {
        self.scene.draft.clear()
    }

    /// Replaces render annotations from version-1 scene JSON.
    ///
    /// This does not load an API share document. The host translates its plan into
    /// render objects and supplies labels and structure sprites separately.
    ///
    /// # Errors
    ///
    /// Uses [`Scene::import`] validation; failure preserves the current scene.
    pub fn import_document(&mut self, json: &str) -> Result<(), JsValue> {
        self.scene.import(json).map_err(error)
    }

    /// Returns missing tile requests as JSON, ordered from coarse to fine.
    ///
    /// Each request contains `key`, `level`, `x`, `y` and `url`. This does not mark
    /// requests as pending; the host deduplicates in-flight fetches. Uploaded-image
    /// maps return an empty array.
    pub fn requests(&self) -> Result<String, JsValue> {
        let tiles: Vec<_> = self
            .scene
            .needed_tiles()
            .into_iter()
            .filter(|tile| !self.gpu.has(*tile))
            .map(|tile| {
                serde_json::json!({
                    "key": tile.key(),
                    "level": tile.level,
                    "x": tile.x,
                    "y": tile.y,
                    "url": format!("{}/{}.webp", self.scene.config.tile_base_url, tile.key()),
                })
            })
            .collect();
        serde_json::to_string(&tiles).map_err(error)
    }

    /// Uploads RGBA8 tile pixels, including overlap at interior tile edges.
    ///
    /// Already resident tiles and responses no longer needed by the current camera
    /// are ignored after validation. Needed ancestor tiles are protected from eviction.
    ///
    /// # Errors
    ///
    /// Rejects invalid tile coordinates, mismatched pixel lengths, or an upload
    /// that cannot fit without evicting a currently needed tile.
    pub fn upload_tile(
        &mut self,
        level: u32,
        x: u32,
        y: u32,
        pixels: &[u8],
    ) -> Result<(), JsValue> {
        self.gpu.upload(Tile { level, x, y }, pixels, &self.scene.needed_tiles()).map_err(error)
    }

    /// Replaces the rendered forbidden geometry from a ROKB mesh container.
    ///
    /// This affects rendering only; navigation owns its own terrain grid.
    ///
    /// # Errors
    ///
    /// Invalid containers or geometry exceeding the decoder's budgets preserve the
    /// current mesh. Container integrity is checked here; the host verifies its
    /// catalog digest before calling this method.
    pub fn set_forbidden(&mut self, mut bytes: Vec<u8>) -> Result<(), JsValue> {
        let triangles = crate::forbidden::decode_triangles(&mut bytes).map_err(error)?;
        self.gpu.forbidden = triangles;
        Ok(())
    }

    /// Draws one frame when the host determines that the view has changed.
    ///
    /// A temporarily unavailable surface can skip a frame successfully. This method
    /// does not schedule a retry or an animation loop.
    ///
    /// # Errors
    ///
    /// Reports retained device errors or a lost surface that requires recreating
    /// the renderer.
    pub fn render(&mut self) -> Result<(), JsValue> {
        self.gpu.render(&self.scene).map_err(error)
    }

    /// Draws an export frame with detail imagery and zoom fades disabled.
    ///
    /// The host chooses dimensions and framing, removes previews and selection
    /// highlights, then reads the canvas pixels. This method neither restores the
    /// editing camera nor encodes an image.
    ///
    /// # Errors
    ///
    /// Has the same device/surface failure behavior as [`Self::render`].
    pub fn render_export(&mut self) -> Result<(), JsValue> {
        self.gpu.render_export(&self.scene).map_err(error)
    }

    /// Returns camera, selection and tile-cache telemetry as JSON.
    ///
    /// `scale` is CSS pixels per world unit; `zoom` is relative to the fitted view.
    /// `textureBytes` counts tile pixels only, excluding full-map images, sprites,
    /// labels and render targets. `frames` counts presented frames, while uploads
    /// and drawn tiles are tracked separately.
    pub fn status(&self) -> Result<String, JsValue> {
        let status = serde_json::json!({
            "backend": self.gpu.backend,
            "scale": self.scene.scale,
            "level": self.scene.level(),
            "zoom": self.scene.scale / self.scene.fit_scale(),
            "center": self.scene.center,
            "objects": self.scene.objects.len(),
            "selected": self.scene.selected,
            "cachedTiles": self.gpu.count(),
            "cacheLimit": CACHE_LIMIT,
            "textureBytes": self.gpu.bytes(),
            "uploadedTiles": self.gpu.uploaded,
            "drawnTiles": self.gpu.drawn,
            "frames": self.gpu.frames,
        });
        serde_json::to_string(&status).map_err(error)
    }
}

/// Worker-compatible forbidden-mesh navigation; it does not allocate a GPU device.
#[wasm_bindgen]
pub struct Pathfinder(crate::Navigation);

#[wasm_bindgen]
impl Pathfinder {
    /// Builds a navigation grid from JSON `[west, south, east, north]` bounds and a mesh.
    ///
    /// # Errors
    ///
    /// Rejects malformed bounds or any input rejected by [`crate::Navigation::from_container`].
    #[wasm_bindgen(constructor)]
    pub fn new(bounds: &str, mut bytes: Vec<u8>) -> Result<Pathfinder, JsValue> {
        let bounds = serde_json::from_str(bounds).map_err(error)?;
        crate::Navigation::from_container(bounds, &mut bytes).map(Self).map_err(error)
    }

    /// Returns a JSON object with `path`, `start` and `end` in native map units.
    ///
    /// Blocked site centers can be replaced by nearby approaches. All three fields
    /// are `null` when no route is found or the bounded search cannot finish. This
    /// is a normal result, not a JavaScript exception.
    pub fn route(&self, ax: f32, ay: f32, bx: f32, by: f32) -> Result<String, JsValue> {
        let path = self.0.route_sites([ax, ay], [bx, by]);
        let start = path.as_ref().and_then(|p| p.first()).copied();
        let end = path.as_ref().and_then(|p| p.last()).copied();
        serde_json::to_string(&serde_json::json!({"path":path,"start":start,"end":end}))
            .map_err(error)
    }

    /// Replaces allowed pass crossings from a JSON array of native `[x, y]` centers.
    ///
    /// # Errors
    ///
    /// Rejects payloads larger than 16 KiB, invalid JSON or centers rejected by
    /// [`crate::Navigation::set_passes`]. Failure preserves the previous crossings.
    pub fn set_passes(&mut self, json: &str) -> Result<(), JsValue> {
        if json.len() > 16_384 {
            return Err(error("Too many navigation passes"));
        }
        let passes: Vec<[f32; 2]> = serde_json::from_str(json).map_err(error)?;
        self.0.set_passes(&passes).map_err(error)
    }
}
