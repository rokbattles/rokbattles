//! Camera, tile selection and annotation state, independent of the GPU.
//!
//! World coordinates increase east and north. Pointer input and annotation sizes
//! use CSS pixels; device pixel ratio affects tile resolution and the canvas only.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// Map bounds and an optional tile pyramid. Omit the pyramid fields for uploaded images.
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Display name from the map response.
    pub name: String,
    /// World extents in west, south, east, north order.
    pub bounds: [f32; 4],
    /// Full-map tile-pyramid resolution; zero selects uploaded overview/detail images.
    #[serde(default)]
    pub image_size: u32,
    /// Tile core size in pixels, excluding overlap.
    #[serde(default)]
    pub tile_size: u32,
    /// Extra border texels used for filtering at tile seams.
    #[serde(default)]
    pub overlap: u32,
    /// URL prefix for `{level}/{x}_{y}.webp` requests.
    #[serde(default)]
    pub tile_base_url: String,
}

impl Config {
    /// Checks finite, increasing bounds and any supplied tile pyramid.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid bounds or a pyramid other than 8192/512/1.
    pub fn validate(&self) -> Result<(), String> {
        let [x0, y0, x1, y1] = self.bounds;
        if !self.bounds.iter().all(|n| n.is_finite())
            || x1 <= x0
            || y1 <= y0
            || !(x1 - x0).is_finite()
            || !(y1 - y0).is_finite()
        {
            return Err("Invalid map bounds".into());
        }
        if self.image_size == 0
            && self.tile_size == 0
            && self.overlap == 0
            && self.tile_base_url.is_empty()
        {
            return Ok(());
        }
        if self.image_size != 8192 || self.tile_size != 512 || self.overlap != 1 {
            return Err("Unsupported tile pyramid; expected 8192 / 512 / 1".into());
        }
        Ok(())
    }
}

/// Identifies one tile in the supported square pyramid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Tile {
    /// Zero-based resolution level; level zero covers the map with one tile.
    pub level: u32,
    /// Tile column, increasing eastward.
    pub x: u32,
    /// Tile row, increasing southward.
    pub y: u32,
}

impl Tile {
    /// Returns the tile path without its file extension.
    pub fn key(self) -> String {
        format!("{}/{}_{}", self.level, self.x, self.y)
    }

    /// Returns encoded pixel dimensions, including overlap except at map edges.
    /// Call only for a valid tile key.
    pub fn dimensions(self) -> [u32; 2] {
        let size = 512u32 << self.level;
        [self.x, self.y].map(|i| (size.min((i + 1) * 512 + 1)) - (i * 512).saturating_sub(1))
    }

    /// Returns whether the level and row/column identify a tile in this pyramid.
    pub fn valid(self) -> bool {
        (0..=4).contains(&self.level) && self.x < (1 << self.level) && self.y < (1 << self.level)
    }
}

/// Annotation appearance using the planner’s shared color and size presets.
#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct Style {
    /// Alliance palette ID, from 1 through 8.
    pub color: u8,
    /// Preset size, from 1 through 3.
    pub size: u8,
}

impl Default for Style {
    fn default() -> Self {
        Self { color: 5, size: 2 }
    }
}

impl Style {
    /// Returns whether both preset IDs are supported.
    pub fn valid(self) -> bool {
        (1..=8).contains(&self.color) && (1..=3).contains(&self.size)
    }

    /// Returns the marker/text scale associated with the size preset.
    pub fn factor(self) -> f32 {
        match self.size {
            1 => 0.8,
            3 => 1.3,
            _ => 1.0,
        }
    }

    /// Returns the palette color as normalized display-space RGBA with full opacity.
    pub fn rgba(self) -> [f32; 4] {
        let rgb = match self.color {
            1 => [255, 0, 0],
            2 => [255, 138, 0],
            3 => [255, 252, 0],
            4 => [0, 255, 90],
            5 => [0, 222, 255],
            6 => [0, 132, 255],
            7 => [180, 0, 255],
            _ => [255, 77, 190],
        };
        [rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0, 1.0]
    }
}

/// Interpretation of a drawing’s world-coordinate control points.
#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    #[default]
    /// A polyline containing every sampled point.
    Freehand,
    /// Two points: shaft start and arrow tip.
    Arrow,
    /// Two points: center and an endpoint defining the radius.
    Circle,
}

/// A planning annotation serialized with its `kind` discriminator.
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Object {
    /// A point marker with a screen-sized symbol.
    Marker {
        id: u32,
        x: f32,
        y: f32,
        #[serde(default)]
        style: Style,
    },
    /// A printable-ASCII label anchored in world coordinates.
    Text {
        id: u32,
        x: f32,
        y: f32,
        text: String,
        #[serde(default)]
        style: Style,
    },
    /// A polyline or two-control-point shape.
    Drawing {
        id: u32,
        points: Vec<[f32; 2]>,
        #[serde(default)]
        shape: Shape,
        #[serde(default)]
        style: Style,
    },
}

impl Object {
    /// Returns the annotation's palette and size presets.
    pub fn style(&self) -> Style {
        match self {
            Self::Marker { style, .. } | Self::Text { style, .. } | Self::Drawing { style, .. } => {
                *style
            }
        }
    }

    fn set_style(&mut self, value: Style) {
        match self {
            Self::Marker { style, .. } | Self::Text { style, .. } | Self::Drawing { style, .. } => {
                *style = value
            }
        }
    }

    /// Returns the stable ID used for selection within this scene document.
    pub fn id(&self) -> u32 {
        match self {
            Self::Marker { id, .. } | Self::Text { id, .. } | Self::Drawing { id, .. } => *id,
        }
    }

    fn points(&self) -> Vec<[f32; 2]> {
        match self {
            Self::Marker { x, y, .. } | Self::Text { x, y, .. } => vec![[*x, *y]],
            Self::Drawing { points, .. } => points.clone(),
        }
    }

    fn translate(&mut self, dx: f32, dy: f32) {
        match self {
            Self::Marker { x, y, .. } | Self::Text { x, y, .. } => {
                *x += dx;
                *y += dy;
            }
            Self::Drawing { points, .. } => {
                for p in points {
                    p[0] += dx;
                    p[1] += dy;
                }
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
struct Document {
    version: u32,
    objects: Vec<Object>,
}

/// Camera, annotations and drawing state shared by the renderer and native tools.
///
/// Use the mutation methods to preserve coordinate and document limits. The public
/// fields expose render state; changing them directly bypasses validation.
pub struct Scene {
    /// Map bounds and tile-source metadata.
    pub config: Config,
    /// Committed annotations in paint order.
    pub objects: Vec<Object>,
    /// IDs of selected annotations.
    pub selected: Vec<u32>,
    /// Style applied to newly created annotations.
    pub brush: Style,
    /// Interpretation of the active draft’s control points.
    pub draft_shape: Shape,
    /// Camera center in world coordinates.
    pub center: [f32; 2],
    /// CSS pixels per world unit.
    pub scale: f32,
    /// Viewport width in CSS pixels.
    pub width: f32,
    /// Viewport height in CSS pixels.
    pub height: f32,
    /// Device pixel ratio, capped at two to bound rendering cost.
    pub dpr: f32,
    /// Uncommitted control points in world coordinates.
    pub draft: Vec<[f32; 2]>,
    next_id: u32,
}

impl Scene {
    /// Decodes a ROKB JSON scene and initializes a camera fitted to the map.
    ///
    /// The container reader decodes the supplied bytes in place.
    ///
    /// # Errors
    ///
    /// Rejects invalid configuration, damaged containers and unsupported documents.
    pub fn new(config: Config, bytes: &mut [u8]) -> Result<Self, String> {
        config.validate()?;
        let decoded =
            rokbattles_container::Reader::default().decode(bytes).map_err(|e| e.to_string())?;
        let rokbattles_container::Value::Json(value) = decoded.value else {
            return Err("Expected a JSON scene container".into());
        };
        let doc: Document = serde_json::from_value(value).map_err(|e| e.to_string())?;
        let mut scene = Self::empty(config)?;
        scene.replace_document(doc)?;
        Ok(scene)
    }

    /// Creates an empty scene fitted to an 800×600 CSS-pixel viewport.
    ///
    /// Call [`Self::resize`] before rendering into a differently sized canvas.
    ///
    /// # Errors
    ///
    /// Returns an error if [`Config::validate`] rejects the map bounds or pyramid.
    pub fn empty(config: Config) -> Result<Self, String> {
        config.validate()?;
        let mut scene = Self {
            config,
            objects: Vec::new(),
            selected: Vec::new(),
            brush: Style::default(),
            draft_shape: Shape::Freehand,
            center: [0.0; 2],
            scale: 1.0,
            width: 1.0,
            height: 1.0,
            dpr: 1.0,
            draft: Vec::new(),
            next_id: 1,
        };
        scene.resize(800.0, 600.0, 1.0);
        scene.fit();
        Ok(scene)
    }

    fn inside(&self, p: [f32; 2]) -> bool {
        let [x0, y0, x1, y1] = self.config.bounds;
        p[0].is_finite() && p[1].is_finite() && p[0] >= x0 && p[0] <= x1 && p[1] >= y0 && p[1] <= y1
    }

    // Validate the entire replacement before mutating state so failed imports are atomic.
    fn replace_document(&mut self, doc: Document) -> Result<(), String> {
        if doc.version != 1 || doc.objects.len() > 2000 {
            return Err("Unsupported document or more than 2000 objects".into());
        }
        if doc.objects.iter().map(|o| o.points().len()).sum::<usize>() > 32768 {
            return Err("Scene exceeds 32768 annotation points".into());
        }
        let mut ids = HashSet::new();
        for o in &doc.objects {
            if !o.style().valid()
                || o.id() == 0
                || o.id() > 1_000_000
                || !ids.insert(o.id())
                || !o.points().iter().all(|p| self.inside(*p))
            {
                return Err("Invalid object coordinates or IDs".into());
            }
            match o {
                Object::Text { text, .. } if !valid_text(text) => {
                    return Err("Labels need 1–64 printable ASCII characters".into());
                }
                Object::Drawing { points, .. } if points.len() < 2 || points.len() > 4096 => {
                    return Err("Drawings need 2–4096 points".into());
                }
                Object::Drawing { points, shape, .. }
                    if *shape != Shape::Freehand && points.len() != 2 =>
                {
                    return Err("Shapes need two control points".into());
                }
                _ => {}
            }
        }
        self.next_id = doc.objects.iter().map(Object::id).max().unwrap_or(0) + 1;
        self.objects = doc.objects;
        self.selected.clear();
        self.draft.clear();
        Ok(())
    }

    /// Replaces annotations from version-1 scene JSON.
    ///
    /// A successful import clears selection and the drawing draft, keeping the camera.
    /// Documents allow up to 2,000 objects and 32,768 control points. IDs must be
    /// unique integers in `1..=1_000_000`, and control points must lie within the map.
    /// Drawings need 2–4,096 points; arrows and circles need exactly two.
    ///
    /// # Errors
    ///
    /// Invalid JSON, unsupported versions, invalid styles or text, and exceeded
    /// coordinate or document limits leave the existing scene intact.
    pub fn import(&mut self, json: &str) -> Result<(), String> {
        let doc = serde_json::from_str(json).map_err(|e| e.to_string())?;
        self.replace_document(doc)
    }

    /// Serializes annotations as version-1 scene JSON, excluding camera and selection state.
    pub fn export(&self) -> Result<String, String> {
        serde_json::to_string(&Document { version: 1, objects: self.objects.clone() })
            .map_err(|e| e.to_string())
    }

    /// Returns the CSS pixels per world unit needed to fit the map with a small margin.
    pub fn fit_scale(&self) -> f32 {
        let [x0, y0, x1, y1] = self.config.bounds;
        (self.width / (x1 - x0)).min(self.height / (y1 - y0)) * 0.94
    }

    /// Centers the map and resets magnification to the fitted view.
    pub fn fit(&mut self) {
        let [x0, y0, x1, y1] = self.config.bounds;
        self.center = [(x0 + x1) * 0.5, (y0 + y1) * 0.5];
        self.scale = self.fit_scale();
    }

    /// Updates the viewport, preserving a fitted view or constraining a zoomed camera.
    ///
    /// CSS dimensions are clamped to `1..=8192` and device pixel ratio to `1..=2`.
    /// Non-finite inputs become one. A camera within 1% of its fitted scale is
    /// fitted again; other cameras keep their scale subject to the new bounds.
    pub fn resize(&mut self, w: f32, h: f32, dpr: f32) {
        let fit = self.scale <= self.fit_scale() * 1.01;
        self.width = if w.is_finite() { w.clamp(1.0, 8192.0) } else { 1.0 };
        self.height = if h.is_finite() { h.clamp(1.0, 8192.0) } else { 1.0 };
        self.dpr = if dpr.is_finite() { dpr.clamp(1.0, 2.0) } else { 1.0 };
        if fit {
            self.fit();
        } else {
            self.constrain();
        }
    }

    /// Converts canvas-relative CSS pixels to east/north world coordinates.
    ///
    /// The result is not clamped to the map; pointer input can lie in its margins.
    pub fn screen_to_world(&self, x: f32, y: f32) -> [f32; 2] {
        [
            self.center[0] + (x - self.width / 2.0) / self.scale,
            self.center[1] - (y - self.height / 2.0) / self.scale,
        ]
    }

    /// Converts world coordinates to canvas-relative CSS pixels.
    pub fn world_to_screen(&self, p: [f32; 2]) -> [f32; 2] {
        [
            (p[0] - self.center[0]) * self.scale + self.width / 2.0,
            (self.center[1] - p[1]) * self.scale + self.height / 2.0,
        ]
    }

    /// Moves the camera by a drag delta measured in CSS pixels.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        if dx.is_finite() && dy.is_finite() {
            self.center[0] -= dx / self.scale;
            self.center[1] += dy / self.scale;
            self.constrain()
        }
    }

    /// Multiplies magnification around a canvas-relative CSS pixel position.
    ///
    /// Magnification is clamped between the fitted scale and 32 times that scale.
    /// Non-finite input or a non-positive factor is ignored. The pointer's world
    /// position stays fixed unless keeping the camera inside the map requires a shift.
    pub fn zoom(&mut self, factor: f32, x: f32, y: f32) {
        if !factor.is_finite() || factor <= 0.0 || !x.is_finite() || !y.is_finite() {
            return;
        }
        // Preserve the world point under the pointer while changing magnification.
        let before = self.screen_to_world(x, y);
        self.scale = (self.scale * factor).clamp(self.fit_scale(), self.fit_scale() * 32.0);
        let after = self.screen_to_world(x, y);
        self.center[0] += before[0] - after[0];
        self.center[1] += before[1] - after[1];
        self.constrain();
    }

    fn constrain(&mut self) {
        let [x0, y0, x1, y1] = self.config.bounds;
        self.scale = self.scale.clamp(self.fit_scale(), self.fit_scale() * 32.0);
        for (center, (lo, hi, screen)) in
            self.center.iter_mut().zip([(x0, x1, self.width), (y0, y1, self.height)])
        {
            let half = screen / self.scale / 2.0;
            *center = if half * 2.0 >= hi - lo {
                (lo + hi) / 2.0
            } else {
                center.clamp(lo + half, hi - half)
            }
        }
    }

    /// Chooses a tile level from 0 (512 pixels) through 4 (8192 pixels).
    ///
    /// Resolution follows projected map width and device pixel ratio, then drops
    /// if the visible tiles and their ancestors would exceed 64 cache entries.
    #[expect(clippy::cast_sign_loss, reason = "Negative log2 values saturate to level zero")]
    pub fn level(&self) -> u32 {
        let span = self.config.bounds[2] - self.config.bounds[0];
        let mut level = ((span * self.scale * self.dpr / 512.0).log2().ceil() as u32).min(4);
        // Keep visible ancestors resident as fallbacks while sharper tiles are fetched.
        // Leave headroom beneath the GPU cache limit for tiles from the previous view.
        while level > 0 && (0..=level).map(|l| self.visible_tiles(l).len()).sum::<usize>() > 64 {
            level -= 1
        }
        level
    }

    /// Returns viewport tiles in row order for a supported level (`0..=4`).
    ///
    /// Callers must supply a supported level; this method does not validate it.
    #[expect(
        clippy::cast_sign_loss,
        reason = "Tile coordinates are clamped to zero before conversion"
    )]
    pub fn visible_tiles(&self, level: u32) -> Vec<Tile> {
        let [x0, y0, x1, y1] = self.config.bounds;
        let n = 1u32 << level;
        let a = self.screen_to_world(0.0, 0.0);
        let b = self.screen_to_world(self.width, self.height);
        let minx = (((a[0] - x0) / (x1 - x0) * n as f32).floor().max(0.0) as u32).min(n - 1);
        let maxx = (((b[0] - x0) / (x1 - x0) * n as f32).floor().max(0.0) as u32).min(n - 1);
        // Tile rows run southward from the north edge of the map.
        let miny = (((y1 - a[1]) / (y1 - y0) * n as f32).floor().max(0.0) as u32).min(n - 1);
        let maxy = (((y1 - b[1]) / (y1 - y0) * n as f32).floor().max(0.0) as u32).min(n - 1);
        (miny..=maxy).flat_map(|y| (minx..=maxx).map(move |x| Tile { level, x, y })).collect()
    }

    /// Lists visible tiles from coarse to fine, including fallback levels.
    ///
    /// Returns an empty list when the map uses uploaded overview/detail images.
    pub fn needed_tiles(&self) -> Vec<Tile> {
        if self.config.image_size == 0 {
            return Vec::new();
        }
        (0..=self.level()).flat_map(|l| self.visible_tiles(l)).collect()
    }

    fn allocate(&mut self) -> Result<u32, String> {
        if self.objects.len() >= 2000 || self.next_id > 1_000_000 {
            return Err("Scene object limit reached".into());
        }
        let id = self.next_id;
        self.next_id += 1;
        Ok(id)
    }

    /// Places a marker or ASCII label at a canvas-relative CSS pixel position.
    ///
    /// `kind` must be `"marker"` or `"text"`. The new object uses the current brush
    /// and becomes the only selected object.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown kind, an out-of-bounds point, a label that
    /// is not 1–64 printable ASCII characters with non-whitespace content, or an
    /// exhausted object/ID budget.
    pub fn add(&mut self, kind: &str, x: f32, y: f32, text: &str) -> Result<(), String> {
        let p = self.screen_to_world(x, y);
        if !self.inside(p) {
            return Err("Place annotations inside the map".into());
        }
        if kind != "marker" && kind != "text" {
            return Err("Unknown annotation type".into());
        }
        if kind == "text" && !valid_text(text) {
            return Err("Use 1–64 printable ASCII characters".into());
        }
        let id = self.allocate()?;
        self.objects.push(if kind == "text" {
            Object::Text { id, x: p[0], y: p[1], text: text.trim().into(), style: self.brush }
        } else {
            Object::Marker { id, x: p[0], y: p[1], style: self.brush }
        });
        self.selected = vec![id];
        Ok(())
    }

    /// Picks the topmost annotation at a canvas-relative CSS pixel position.
    ///
    /// Additive selection toggles the hit object's membership. Otherwise a hit
    /// replaces the selection, except that hitting an already selected object
    /// preserves the group. An empty hit clears only non-additive selections.
    pub fn select(&mut self, x: f32, y: f32, additive: bool) {
        let p = [x, y];
        let hit = self
            .objects
            .iter()
            .rev()
            .find(|o| match o {
                Object::Marker { x, y, .. } => {
                    distance(p, self.world_to_screen([*x, *y])) < 16.0 * o.style().factor()
                }
                Object::Text { x, y, text, .. } => {
                    let a = self.world_to_screen([*x, *y]);
                    p[0] >= a[0] - 4.0
                        && p[0] <= a[0] + text.len() as f32 * 12.0 * o.style().factor() + 4.0
                        && (p[1] - a[1]).abs() < 16.0 * o.style().factor()
                }
                Object::Drawing { points, shape, .. } => {
                    shape_points(points, *shape).array_windows::<2>().any(|pair| {
                        segment_distance(
                            p,
                            self.world_to_screen(pair[0]),
                            self.world_to_screen(pair[1]),
                        ) < 9.0
                    })
                }
            })
            .map(Object::id);
        if additive {
            if let Some(id) = hit {
                if self.selected.contains(&id) {
                    self.selected.retain(|v| *v != id)
                } else {
                    self.selected.push(id)
                }
            }
        } else if let Some(id) = hit {
            if !self.selected.contains(&id) {
                self.selected = vec![id]
            }
        } else {
            self.selected.clear()
        }
    }

    /// Selects all committed annotations.
    pub fn select_all(&mut self) {
        self.selected = self.objects.iter().map(Object::id).collect()
    }

    /// Sets the brush and selected objects to a palette color (1–8) and size (1–3).
    ///
    /// # Errors
    ///
    /// Invalid color or size IDs leave both brush and selection styles unchanged.
    pub fn set_style(&mut self, color: u8, size: u8) -> Result<(), String> {
        let style = Style { color, size };
        if !style.valid() {
            return Err("Choose a preset color and size 1, 2 or 3".into());
        }
        self.brush = style;
        for o in &mut self.objects {
            if self.selected.contains(&o.id()) {
                o.set_style(style)
            }
        }
        Ok(())
    }

    /// Translates the selection by a CSS pixel delta if all points remain inside the map.
    pub fn move_selected(&mut self, dx: f32, dy: f32) {
        if !dx.is_finite() || !dy.is_finite() {
            return;
        }
        let dx = dx / self.scale;
        let dy = -dy / self.scale;
        // Reject the whole translation if any selected point would leave the map.
        if self
            .objects
            .iter()
            .filter(|o| self.selected.contains(&o.id()))
            .all(|o| o.points().iter().all(|p| self.inside([p[0] + dx, p[1] + dy])))
        {
            for o in &mut self.objects {
                if self.selected.contains(&o.id()) {
                    o.translate(dx, dy)
                }
            }
        }
    }

    /// Removes selected annotations and clears the selection.
    pub fn delete(&mut self) {
        self.objects.retain(|o| !self.selected.contains(&o.id()));
        self.selected.clear()
    }

    /// Replaces the draft with a freehand, arrow or circle starting point.
    ///
    /// `kind` must be `"drawing"`, `"arrow"` or `"circle"`. Coordinates use CSS
    /// pixels. An out-of-bounds starting point leaves the new draft empty.
    ///
    /// # Errors
    ///
    /// An unknown kind leaves the previous draft and shape unchanged.
    pub fn begin_drawing(&mut self, kind: &str, x: f32, y: f32) -> Result<(), String> {
        self.draft_shape = match kind {
            "drawing" => Shape::Freehand,
            "arrow" => Shape::Arrow,
            "circle" => Shape::Circle,
            _ => return Err("Unknown drawing tool".into()),
        };
        self.draft.clear();
        self.draw_point(x, y);
        Ok(())
    }

    /// Extends a draft with a point more than two CSS pixels from its last point.
    ///
    /// Arrow and circle drafts replace their second point instead of growing a
    /// polyline. Out-of-bounds input and input beyond the 4,096-point cap are ignored.
    pub fn draw_point(&mut self, x: f32, y: f32) {
        let p = self.screen_to_world(x, y);
        if self.inside(p)
            && self.draft.len() < 4096
            && self.draft.last().is_none_or(|last| distance(*last, p) * self.scale > 2.0)
        {
            match (self.draft_shape, self.draft.as_mut_slice()) {
                (Shape::Arrow | Shape::Circle, [_, endpoint]) => *endpoint = p,
                _ => self.draft.push(p),
            }
        }
    }

    /// Commits a draft with at least two points and selects the new drawing.
    ///
    /// Shorter drafts are discarded without creating an object.
    ///
    /// # Errors
    ///
    /// Exceeding the scene's point budget discards the draft. An exhausted object
    /// or ID budget leaves the draft intact.
    pub fn finish_drawing(&mut self) -> Result<(), String> {
        if self.draft.len() > 1 {
            if self.objects.iter().map(|o| o.points().len()).sum::<usize>() + self.draft.len()
                > 32768
            {
                self.draft.clear();
                return Err("Scene exceeds 32768 annotation points".into());
            }
            let id = self.allocate()?;
            self.objects.push(Object::Drawing {
                id,
                points: std::mem::take(&mut self.draft),
                shape: self.draft_shape,
                style: self.brush,
            });
            self.selected = vec![id]
        } else {
            self.draft.clear()
        }
        Ok(())
    }
}

/// Expands a circle into a closed 64-segment polyline shared by drawing and picking.
pub fn shape_points(points: &[[f32; 2]], shape: Shape) -> Vec<[f32; 2]> {
    if shape != Shape::Circle {
        return points.to_vec();
    }
    let [a, endpoint] = points else {
        return points.to_vec();
    };
    let radius = distance(*a, *endpoint);
    (0..=64)
        .map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / 64.0;
            [a[0] + angle.cos() * radius, a[1] + angle.sin() * radius]
        })
        .collect()
}

// The browser builds a fixed atlas for printable ASCII; other glyphs have no cell.
fn valid_text(text: &str) -> bool {
    !text.trim().is_empty() && text.len() <= 64 && text.bytes().all(|c| (32..=126).contains(&c))
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn segment_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let t = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1])
        / (d[0] * d[0] + d[1] * d[1]).max(0.0001))
    .clamp(0.0, 1.0);
    distance(p, [a[0] + t * d[0], a[1] + t * d[1]])
}

#[cfg(test)]
#[expect(
    clippy::panic_in_result_fn,
    reason = "Tests return setup errors and use assertions for behavior"
)]
mod tests {
    use super::*;

    #[test]
    fn uploaded_maps_do_not_request_tiles() -> Result<(), Box<dyn std::error::Error>> {
        let config = serde_json::from_str(r#"{"name":"Home 1","bounds":[0,0,7200,7200]}"#)?;
        let mut scene = Scene::empty(config)?;
        scene.zoom(8.0, 400.0, 300.0);
        scene.add("marker", 400.0, 300.0, "")?;
        assert!(scene.needed_tiles().is_empty());
        assert_eq!(scene.objects.len(), 1);
        Ok(())
    }

    #[test]
    fn config_rejects_a_non_finite_derived_span() {
        let config = Config {
            name: "Invalid".into(),
            bounds: [-f32::MAX, 0.0, f32::MAX, 1.0],
            image_size: 0,
            tile_size: 0,
            overlap: 0,
            tile_base_url: String::new(),
        };
        assert!(config.validate().is_err());
    }

    fn scene() -> Result<Scene, Box<dyn std::error::Error>> {
        let c = Config {
            name: "Synthetic tiled map".into(),
            bounds: [0.0, 0.0, 720.0, 720.0],
            image_size: 8192,
            tile_size: 512,
            overlap: 1,
            tile_base_url: "/test-tiles".into(),
        };
        let mut bytes =
            rokbattles_container::write_json(&serde_json::json!({"version":1,"objects":[]}), 42)?;
        Ok(Scene::new(c, &mut bytes)?)
    }

    #[test]
    fn zoom_keeps_pointer_world_position() -> Result<(), Box<dyn std::error::Error>> {
        let mut s = scene()?;
        s.zoom(4.0, 400.0, 300.0);
        let before = s.screen_to_world(420.0, 320.0);
        s.zoom(1.5, 420.0, 320.0);
        let after = s.screen_to_world(420.0, 320.0);
        assert!(distance(before, after) < 0.001);
        Ok(())
    }

    #[test]
    fn tile_level_accounts_for_pixel_density() -> Result<(), Box<dyn std::error::Error>> {
        let mut s = scene()?;
        s.resize(800.0, 600.0, 2.0);
        s.fit();
        assert_eq!(s.level(), 2);
        s.zoom(32.0, 400.0, 300.0);
        assert_eq!(s.level(), 4);
        assert!(s.needed_tiles().len() < 80);
        Ok(())
    }

    #[test]
    fn tile_uv_overlap_has_expected_edge_sizes() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(Tile { level: 4, x: 0, y: 0 }.dimensions(), [513, 513]);
        assert_eq!(Tile { level: 4, x: 1, y: 1 }.dimensions(), [514, 514]);
        assert_eq!(Tile { level: 0, x: 0, y: 0 }.dimensions(), [512, 512]);
        Ok(())
    }

    #[test]
    fn annotations_roundtrip_move_and_delete() -> Result<(), Box<dyn std::error::Error>> {
        let mut s = scene()?;
        s.add("text", 400.0, 300.0, "Rally here")?;
        s.select(410.0, 300.0, false);
        assert!(!s.selected.is_empty());
        s.move_selected(10.0, 20.0);
        let doc = s.export()?;
        let mut other = scene()?;
        other.import(&doc)?;
        assert_eq!(other.export()?, doc);
        s.delete();
        assert!(s.objects.is_empty());
        Ok(())
    }

    #[test]
    fn drawing_hit_test_selects_segment() -> Result<(), Box<dyn std::error::Error>> {
        let mut s = scene()?;
        s.draw_point(300.0, 300.0);
        s.draw_point(500.0, 300.0);
        s.finish_drawing()?;
        s.selected.clear();
        s.select(400.0, 304.0, false);
        assert!(!s.selected.is_empty());
        Ok(())
    }

    #[test]
    fn malformed_import_is_atomic() -> Result<(), Box<dyn std::error::Error>> {
        let mut s = scene()?;
        s.add("marker", 400.0, 300.0, "")?;
        let before = s.export()?;
        assert!(
            s.import(r#"{"version":1,"objects":[{"kind":"marker","id":1,"x":99999,"y":10}]}"#)
                .is_err()
        );
        assert_eq!(s.export()?, before);
        Ok(())
    }

    #[test]
    fn multi_selection_styles_moves_and_bulk_deletes() -> Result<(), Box<dyn std::error::Error>> {
        let mut s = scene()?;
        s.add("marker", 350.0, 300.0, "")?;
        s.add("marker", 450.0, 300.0, "")?;
        s.select(350.0, 300.0, true);
        assert_eq!(s.selected.len(), 2);
        s.set_style(7, 3)?;
        assert!(s.objects.iter().all(|o| o.style().color == 7));
        s.move_selected(0.0, 30.0);
        s.delete();
        assert!(s.objects.is_empty());
        Ok(())
    }

    #[test]
    fn circle_selects_circumference_and_roundtrips() -> Result<(), Box<dyn std::error::Error>> {
        let mut s = scene()?;
        s.begin_drawing("circle", 400.0, 300.0)?;
        s.draw_point(440.0, 300.0);
        s.finish_drawing()?;
        s.selected.clear();
        s.select(400.0, 340.0, false);
        assert_eq!(s.selected.len(), 1);
        let doc = s.export()?;
        s.import(&doc)?;
        assert!(s.export()?.contains("circle"));
        Ok(())
    }

    #[test]
    fn large_viewports_respect_tile_budget() -> Result<(), Box<dyn std::error::Error>> {
        let mut s = scene()?;
        s.resize(8192.0, 8192.0, 2.0);
        s.fit();
        assert!(s.needed_tiles().len() <= 64);
        Ok(())
    }

    #[test]
    fn rejects_damaged_container() -> Result<(), Box<dyn std::error::Error>> {
        let s = scene()?;
        let mut bytes = vec![0; 24];
        assert!(Scene::new(s.config, &mut bytes).is_err());
        Ok(())
    }
}
