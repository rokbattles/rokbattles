//! Renders planner maps and annotations through WebGPU or WebGL2.
//!
//! [`Scene`] manages the camera, tile selection and annotation validation without
//! a browser. [`Navigation`] estimates routes from construction-forbidden terrain.
//! On WebAssembly, the browser adapter uploads scene geometry and textures through
//! `wgpu`. The host owns the plan, input events, asset loading and frame scheduling.
//!
//! # Coordinates and imagery
//!
//! World coordinates use native map units, increasing east and north. Pointer
//! positions, line widths and label sizes use canvas-relative CSS pixels, with
//! the origin at the top left. Device pixel ratio affects the backing canvas and
//! tile resolution; it does not change pointer coordinates.
//!
//! Territory Lab uploads an overview and a detail image covering the same bounds.
//! The detail image and forbidden terrain fade in together between 1× and 2× the
//! fitted view. Sprite overlays can set their own zoom thresholds. The optional
//! tile pyramid uses levels 0–4 for full-map resolutions of 512–8192 pixels, with
//! 512-pixel tile cores and one-pixel overlap. Coarser tiles fill gaps while detail
//! tiles load.
//!
//! Both browser backends blend colors in display space. An sRGB surface needs a
//! final conversion pass to preserve those colors. The host can retry WebGL2 on a
//! fresh canvas if WebGPU initialization fails.
//!
//! # Scene documents
//!
//! The renderer accepts version-1 JSON containing markers, ASCII text and drawings.
//! This is a rendering document, separate from the API's version-2 shared plan.
//! The host supplies Unicode labels as image overlays. [`Scene::new`] also accepts
//! a scene wrapped in a ROKB schema-3 JSON container. Failed imports preserve the
//! current scene; successful imports reset selection and the drawing draft.
//!
//! ```
//! use rokbattles_webgpu_map_engine::{Config, Scene};
//!
//! # fn main() -> Result<(), String> {
//! let config: Config = serde_json::from_str(
//!     r#"{"name":"Example map","bounds":[0,0,600,600]}"#,
//! ).map_err(|error| error.to_string())?;
//! let mut scene = Scene::empty(config)?;
//! scene.resize(800.0, 600.0, 1.0);
//! scene.fit();
//! scene.add("marker", 400.0, 300.0, "")?;
//! let document = scene.export()?;
//! # assert!(document.contains("marker"));
//! # Ok(())
//! # }
//! ```
//!
//! # Navigation
//!
//! [`Navigation`] reduces construction clearance to estimate walking passages and
//! adds explicit crossings through assigned passes. Territory Lab runs this work
//! in a worker. These routes approximate traversable terrain; they do not reproduce
//! the game server's movement mesh or guarantee a continuous shortest path.
//!
//! # Browser build
//!
//! Run `pnpm -F @rokbattles/site generate:wasm` to generate the WASM module
//! and bindings, then start the site and open `/territory-lab`. The build requires
//! the `wasm32-unknown-unknown` target and a `wasm-bindgen-cli` version matching the
//! workspace dependency. WebGPU requires browser support and HTTPS or localhost.

#[cfg(target_arch = "wasm32")]
mod browser;
mod forbidden;
#[cfg(target_arch = "wasm32")]
mod gpu;
mod navigation;
mod scene;
pub use navigation::Navigation;
pub use scene::{Config, Scene};
