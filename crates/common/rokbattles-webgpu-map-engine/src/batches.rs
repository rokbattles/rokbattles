//! Merge adjacent texture draws without changing alpha compositing order.

use std::ops::Range;

pub(crate) fn append<'a>(
    batches: &mut Vec<(&'a str, Range<u32>)>,
    texture: &'a str,
    vertices: Range<u32>,
) {
    if vertices.is_empty() {
        return;
    }

    if let Some((previous_texture, previous_vertices)) = batches.last_mut()
        && *previous_texture == texture
        && previous_vertices.end == vertices.start
    {
        previous_vertices.end = vertices.end;
    } else {
        batches.push((texture, vertices));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_fills_and_flags_share_draws() {
        let mut batches = Vec::new();

        for i in 0..1_000 {
            append(&mut batches, "white", i * 6..(i + 1) * 6);
        }

        append(&mut batches, "flag", 6_000..6_006);
        append(&mut batches, "flag", 6_006..6_012);

        assert_eq!(batches, vec![("white", 0..6_000), ("flag", 6_000..6_012)]);
    }

    #[test]
    fn interleaved_textures_keep_painter_order() {
        let mut batches = Vec::new();

        append(&mut batches, "white", 0..6);
        append(&mut batches, "flag", 6..12);
        append(&mut batches, "white", 12..18);

        assert_eq!(batches, vec![("white", 0..6), ("flag", 6..12), ("white", 12..18)]);
    }

    #[test]
    fn skipped_geometry_cannot_be_included_in_a_merged_draw() {
        let mut batches = Vec::new();

        append(&mut batches, "white", 0..6);
        append(&mut batches, "white", 12..18);

        assert_eq!(batches, vec![("white", 0..6), ("white", 12..18)]);
    }

    #[test]
    fn empty_draws_do_not_split_adjacent_batches() {
        let mut batches = Vec::new();

        append(&mut batches, "white", 0..6);
        append(&mut batches, "flag", 6..6);
        append(&mut batches, "white", 6..12);

        assert_eq!(batches, vec![("white", 0..12)]);
    }
}
