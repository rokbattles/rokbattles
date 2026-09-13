//! Expands reusable forbidden-mesh definitions into world-space triangles.
//!
//! Rendering and navigation share this decoder so both interpret instance transforms
//! identically. Geometry budgets apply to the expanded result as well as the source:
//! a small definition can otherwise allocate a large result through repeated instances.

use std::collections::HashMap;

use serde::Deserialize;

type Point = [f32; 2];

const MAX_CONTAINER_BYTES: usize = 8 * 1024 * 1024;
const MAX_DEFINITIONS: usize = 4_096;
const MAX_VERTICES: usize = 1_000_000;
const MAX_INDICES: usize = 3_000_000;
const MAX_INSTANCES: usize = 100_000;
const MAX_EXPANDED_TRIANGLES: usize = 2_000_000;

#[derive(Deserialize)]
struct Mesh {
    id: usize,
    vertices: Vec<Point>,
    indices: Vec<usize>,
}

#[derive(Deserialize)]
struct Instance {
    mesh: usize,
    affine: [f32; 6],
}

#[derive(Deserialize)]
struct Forbidden {
    definitions: Vec<Mesh>,
    instances: Vec<Instance>,
}

/// Decodes a ROKB JSON mesh and applies each instance's affine transform.
///
/// The container reader may modify `bytes`. Vertex indices address each definition's
/// local vertices; instance references use definition IDs rather than array offsets.
///
/// # Errors
///
/// Rejects malformed containers, exceeded geometry budgets, duplicate or unknown
/// definitions, invalid indices, and non-finite source or transformed coordinates.
pub(crate) fn decode_triangles(bytes: &mut [u8]) -> Result<Vec<[Point; 3]>, String> {
    if bytes.len() > MAX_CONTAINER_BYTES {
        return Err("Forbidden mesh container too large".into());
    }
    let value =
        rokbattles_container::Reader::default().decode(bytes).map_err(|error| error.to_string())?;
    let rokbattles_container::Value::Json(json) = value.value else {
        return Err("Expected forbidden JSON".into());
    };
    let mesh: Forbidden = serde_json::from_value(json).map_err(|error| error.to_string())?;
    if mesh.definitions.len() > MAX_DEFINITIONS
        || mesh.instances.len() > MAX_INSTANCES
        || mesh.definitions.iter().map(|definition| definition.vertices.len()).sum::<usize>()
            > MAX_VERTICES
        || mesh.definitions.iter().map(|definition| definition.indices.len()).sum::<usize>()
            > MAX_INDICES
        || mesh.definitions.iter().any(|definition| {
            definition.indices.len() % 3 != 0
                || definition.vertices.iter().flatten().any(|coordinate| !coordinate.is_finite())
        })
    {
        return Err("Forbidden mesh exceeds geometry limits".into());
    }

    let mut definitions = HashMap::with_capacity(mesh.definitions.len());
    for definition in &mesh.definitions {
        if definitions.insert(definition.id, definition).is_some() {
            return Err("Duplicate forbidden mesh definition".into());
        }
    }
    // Count before allocating; the instance count alone does not bound expansion.
    let triangle_count = mesh.instances.iter().try_fold(0_usize, |count, instance| {
        let definition = definitions.get(&instance.mesh).ok_or("Unknown forbidden mesh")?;
        count
            .checked_add(definition.indices.len() / 3)
            .filter(|expanded| *expanded <= MAX_EXPANDED_TRIANGLES)
            .ok_or("Forbidden mesh exceeds expanded geometry limit")
    })?;

    let mut triangles = Vec::with_capacity(triangle_count);
    for instance in mesh.instances {
        if !instance.affine.iter().all(|coordinate| coordinate.is_finite()) {
            return Err("Invalid mesh transform".into());
        }
        let definition = definitions.get(&instance.mesh).ok_or("Unknown forbidden mesh")?;
        // The stored matrix maps (x, y) to (a*x + b*y + tx, c*x + d*y + ty).
        let [a, b, c, d, tx, ty] = instance.affine;
        for indices in definition.indices.as_chunks::<3>().0 {
            let mut triangle = [[0.0; 2]; 3];
            for (output, index) in triangle.iter_mut().zip(indices) {
                let [x, y] = *definition.vertices.get(*index).ok_or("Invalid triangle index")?;
                *output = [a * x + b * y + tx, c * x + d * y + ty];
            }
            if triangle.iter().flatten().any(|coordinate| !coordinate.is_finite()) {
                return Err("Invalid mesh coordinate".into());
            }
            triangles.push(triangle);
        }
    }
    Ok(triangles)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn container(indices: serde_json::Value) -> Vec<u8> {
        rokbattles_container::write_json(
            &serde_json::json!({
                "definitions": [{"id": 7, "vertices": [[0, 0], [2, 0], [0, 3]], "indices": indices}],
                "instances": [{"mesh": 7, "affine": [2, 0, 0, 2, 10, 20]}]
            }),
            42,
        )
        .expect("fixture should encode")
    }

    #[test]
    fn decoder_should_expand_finite_world_triangles() {
        let triangles = decode_triangles(&mut container(serde_json::json!([0, 1, 2])))
            .expect("mesh should decode");
        assert_eq!(triangles, vec![[[10.0, 20.0], [14.0, 20.0], [10.0, 26.0]]]);
    }

    #[test]
    fn decoder_should_reject_partial_or_invalid_triangles() {
        decode_triangles(&mut container(serde_json::json!([0, 1]))).unwrap_err();
        decode_triangles(&mut container(serde_json::json!([0, 1, 99]))).unwrap_err();
    }
}
