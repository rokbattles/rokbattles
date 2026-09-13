//! Grid navigation over the client's forbidden triangles.
//!
//! Construction triangles are rasterized conservatively, then their clearance
//! is reduced by two six-unit cells to estimate narrow walking passages. This
//! is not the server's movement map. Eight-way A* forbids
//! corner cutting; smoothing only removes waypoints with clear line of sight.
//! This finds short paths on a six-native-unit grid, not a continuous global optimum.

use std::{
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap, HashSet},
};

type Point = [f32; 2];
const MAX_GRID_CELLS: usize = 1_500_000;
// Site centers can lie deep inside a blocked footprint. This radius bounds the
// search for nearby open terrain; it does not permit walking through the footprint.
const APPROACH_RADIUS: f32 = 96.0;
const MAX_PASSES: usize = 128;
const MAX_PATH_POINTS: usize = 8_192;

#[derive(Clone, Copy)]
struct Portal {
    to: usize,
    via: Point,
    cost: u32,
}

/// A bounded walking grid with optional crossings through assigned passes.
///
/// Positions use native map units. West and south bounds are inclusive; east and
/// north bounds are exclusive. Build the terrain once with [`Self::from_container`],
/// replace the route's allowed crossings with [`Self::set_passes`], then query
/// [`Self::route_sites`] for endpoints inside structures or [`Self::route`] for
/// endpoints already on open terrain.
pub struct Navigation {
    bounds: [f32; 4],
    cell: f32,
    width: usize,
    height: usize,
    blocked: Vec<bool>,
    // Zero marks blocked cells; other IDs index region_sizes and connections.
    regions: Vec<usize>,
    region_sizes: Vec<usize>,
    // Union-find parents include pass links without changing the terrain regions.
    connections: Vec<usize>,
    portals: BTreeMap<usize, Vec<Portal>>,
}

#[expect(
    clippy::indexing_slicing,
    clippy::cast_sign_loss,
    reason = "Grid dimensions are validated and coordinates are clamped before indexing or unsigned conversion"
)]
impl Navigation {
    /// Builds a six-unit grid from a ROKB forbidden-mesh container.
    ///
    /// Decoding may modify `bytes`, including on failure. Construction clearance
    /// is reduced for routing only; the source triangles remain suitable for
    /// placement checks and rendering. No pass crossings are enabled initially.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed geometry, invalid bounds, or a grid exceeding
    /// 2,000 cells on either axis or 1,500,000 cells in total.
    pub fn from_container(bounds: [f32; 4], bytes: &mut [u8]) -> Result<Self, String> {
        let triangles = crate::forbidden::decode_triangles(bytes)?;
        let mut grid = Self::empty(bounds, 6.0)?;
        for triangle in triangles {
            grid.block_triangle(triangle);
        }
        grid.reduce_building_clearance();
        grid.index_regions();
        Ok(grid)
    }

    fn empty(bounds: [f32; 4], cell: f32) -> Result<Self, String> {
        if !bounds.iter().all(|n| n.is_finite()) || bounds[2] <= bounds[0] || bounds[3] <= bounds[1]
        {
            return Err("Invalid navigation bounds".into());
        }
        let width = ((bounds[2] - bounds[0]) / cell).ceil() as usize;
        let height = ((bounds[3] - bounds[1]) / cell).ceil() as usize;
        if width == 0
            || height == 0
            || width > 2000
            || height > 2000
            || width.checked_mul(height).is_none_or(|cells| cells > MAX_GRID_CELLS)
        {
            return Err("Navigation map too large".into());
        }
        Ok(Self {
            bounds,
            cell,
            width,
            height,
            blocked: vec![false; width * height],
            regions: vec![1; width * height],
            region_sizes: vec![0, width * height],
            connections: vec![0, 1],
            portals: BTreeMap::new(),
        })
    }

    fn reduce_building_clearance(&mut self) {
        // Reduce only route clearance by 12 native units (2 game units). Placement
        // and rendering retain the original triangles. Cardinal expansion opens
        // narrow construction gaps while preserving wider authored zone barriers.
        for _ in 0..2 {
            // Each pass reads one snapshot so erosion is independent of scan order.
            let previous = self.blocked.clone();
            for y in 1..self.height.saturating_sub(1) {
                for x in 1..self.width.saturating_sub(1) {
                    let id = y * self.width + x;
                    if !previous[id - 1]
                        || !previous[id + 1]
                        || !previous[id - self.width]
                        || !previous[id + self.width]
                    {
                        self.blocked[id] = false;
                    }
                }
            }
        }
    }

    fn index_regions(&mut self) {
        self.regions.fill(0);
        self.region_sizes.clear();
        self.region_sizes.push(0);
        let mut region = 0;
        let mut pending = Vec::new();
        for seed in 0..self.blocked.len() {
            if self.blocked[seed] || self.regions[seed] != 0 {
                continue;
            }
            region += 1;
            self.region_sizes.push(0);
            self.regions[seed] = region;
            pending.push(seed);
            while let Some(id) = pending.pop() {
                self.region_sizes[region] += 1;
                let x = id % self.width;
                let y = id / self.width;
                // Four-way connectivity matches eight-way routing with no corner cutting.
                for next in [
                    (x > 0).then(|| id - 1),
                    (x + 1 < self.width).then_some(id + 1),
                    (y > 0).then(|| id - self.width),
                    (y + 1 < self.height).then_some(id + self.width),
                ]
                .into_iter()
                .flatten()
                {
                    if !self.blocked[next] && self.regions[next] == 0 {
                        self.regions[next] = region;
                        pending.push(next);
                    }
                }
            }
        }
        self.connections = (0..=region).collect();
    }

    fn connected_region(&self, mut region: usize) -> usize {
        while self.connections[region] != region {
            region = self.connections[region];
        }
        region
    }

    /// Replaces the allowed crossings with links through the given pass centers.
    ///
    /// Each pass links the two largest nearby terrain regions without unblocking
    /// the pass footprint. A center with fewer than two nearby regions adds no link;
    /// repeated links are ignored. An empty slice removes all crossings.
    ///
    /// # Errors
    ///
    /// Returns an error for more than 128 centers or a non-finite or out-of-bounds
    /// center. Validation failures preserve the existing crossings.
    pub fn set_passes(&mut self, passes: &[Point]) -> Result<(), String> {
        if passes.len() > MAX_PASSES || passes.iter().any(|p| self.index(*p).is_none()) {
            return Err("Invalid navigation passes".into());
        }
        self.portals.clear();
        self.connections = (0..self.region_sizes.len()).collect();
        let mut portal_cells = HashSet::new();
        for &via in passes {
            let mut sides: Vec<_> = self.nearby_approaches(via, false).into_iter().collect();
            // A pass footprint can contain sizeable open cracks. Prefer the two
            // terrain regions with the greatest total area, using proximity only
            // to choose between equally sized regions. This approximates the two
            // map zones bordering the assigned pass without opening its mesh.
            sides.sort_by(|a, b| {
                self.region_sizes[b.0]
                    .cmp(&self.region_sizes[a.0])
                    .then(a.1.1.total_cmp(&b.1.1))
                    .then(a.0.cmp(&b.0))
            });
            let [a, b, ..] = sides.as_slice() else {
                continue;
            };
            let Some(start) = self.index(a.1.0) else {
                continue;
            };
            let Some(end) = self.index(b.1.0) else {
                continue;
            };
            let edge = if start < end { (start, end) } else { (end, start) };
            if !portal_cells.insert(edge) {
                continue;
            }
            let cost = ((a.1.1.sqrt() + b.1.1.sqrt()) / self.cell * 10.0).ceil() as u32;
            self.portals.entry(start).or_default().push(Portal { to: end, via, cost });
            self.portals.entry(end).or_default().push(Portal { to: start, via, cost });
            let first = self.connected_region(a.0);
            let second = self.connected_region(b.0);
            self.connections[first] = second;
        }
        Ok(())
    }

    fn center(&self, id: usize) -> Point {
        [
            self.bounds[0] + (id % self.width) as f32 * self.cell + self.cell / 2.0,
            self.bounds[1] + (id / self.width) as f32 * self.cell + self.cell / 2.0,
        ]
    }

    fn index(&self, p: Point) -> Option<usize> {
        if !p.iter().all(|n| n.is_finite())
            || p[0] < self.bounds[0]
            || p[1] < self.bounds[1]
            || p[0] >= self.bounds[2]
            || p[1] >= self.bounds[3]
        {
            return None;
        }
        Some(
            ((p[1] - self.bounds[1]) / self.cell) as usize * self.width
                + ((p[0] - self.bounds[0]) / self.cell) as usize,
        )
    }

    fn block_triangle(&mut self, t: [Point; 3]) {
        let min = [0, 1].map(|axis| t.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min));
        let max = [0, 1].map(|axis| t.iter().map(|p| p[axis]).fold(f32::NEG_INFINITY, f32::max));
        let x0 = ((min[0] - self.bounds[0]) / self.cell).floor().max(0.0) as usize;
        let y0 = ((min[1] - self.bounds[1]) / self.cell).floor().max(0.0) as usize;
        let x1 =
            (((max[0] - self.bounds[0]) / self.cell).floor().max(0.0) as usize).min(self.width - 1);
        let y1 = (((max[1] - self.bounds[1]) / self.cell).floor().max(0.0) as usize)
            .min(self.height - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let id = y * self.width + x;
                let p = self.center(id);
                // Separating axes for the triangle and axis-aligned cell square.
                let separated = (0..3).any(|i| {
                    let a = t[i];
                    let b = t[(i + 1) % 3];
                    let axis = [a[1] - b[1], b[0] - a[0]];
                    let lo = t
                        .iter()
                        .map(|v| v[0] * axis[0] + v[1] * axis[1])
                        .fold(f32::INFINITY, f32::min);
                    let hi = t
                        .iter()
                        .map(|v| v[0] * axis[0] + v[1] * axis[1])
                        .fold(f32::NEG_INFINITY, f32::max);
                    let center = p[0] * axis[0] + p[1] * axis[1];
                    let radius = self.cell / 2.0 * (axis[0].abs() + axis[1].abs());
                    center + radius < lo || center - radius > hi
                });
                if !separated {
                    self.blocked[id] = true;
                }
            }
        }
    }

    /// Returns `p` if it is open, or the nearest open cell center within 96 units.
    ///
    /// Returns `None` for an invalid position or when no approach is nearby. This
    /// lookup does not consider the destination's region; use [`Self::route_sites`]
    /// to select approaches that can be connected.
    pub fn approach(&self, p: Point) -> Option<Point> {
        self.approaches(p).into_values().min_by(|a, b| a.1.total_cmp(&b.1)).map(|(point, _)| point)
    }

    fn approaches(&self, p: Point) -> BTreeMap<usize, (Point, f32)> {
        self.nearby_approaches(p, true)
    }

    fn nearby_approaches(&self, p: Point, use_exact: bool) -> BTreeMap<usize, (Point, f32)> {
        let mut candidates = BTreeMap::new();
        let Some(id) = self.index(p) else {
            return candidates;
        };
        if use_exact && !self.blocked[id] {
            candidates.insert(self.regions[id], (p, 0.0));
            return candidates;
        }
        let radius = (APPROACH_RADIUS / self.cell).ceil() as usize;
        let x = id % self.width;
        let y = id / self.width;
        for row in y.saturating_sub(radius)..=(y + radius).min(self.height - 1) {
            for col in x.saturating_sub(radius)..=(x + radius).min(self.width - 1) {
                let candidate = row * self.width + col;
                if self.blocked[candidate] {
                    continue;
                }
                let q = self.center(candidate);
                let d = (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2);
                let region = self.regions[candidate];
                if d <= APPROACH_RADIUS.powi(2)
                    && candidates.get(&region).is_none_or(|(_, old)| d < *old)
                {
                    candidates.insert(region, (q, d));
                }
            }
        }
        candidates
    }

    /// Estimates a route between sites, substituting nearby approaches when blocked.
    ///
    /// Open endpoints stay in their own terrain region. For blocked endpoints, the
    /// chosen connected pair minimizes the sum of squared approach distances, not
    /// total route length. Returned endpoints can therefore differ from `a` and `b`.
    ///
    /// Returns `None` when no connected approach pair exists or [`Self::route`]
    /// cannot complete the search within its limits.
    pub fn route_sites(&self, a: Point, b: Point) -> Option<Vec<Point>> {
        let starts = self.approaches(a);
        let ends = self.approaches(b);
        let (start, end, _) = starts
            .into_iter()
            .flat_map(|(region, (point, distance))| {
                ends.iter().filter_map(move |(other_region, (end, other))| {
                    (self.connected_region(region) == self.connected_region(*other_region))
                        .then_some((point, *end, distance + other))
                })
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))?;
        self.route(start, end)
    }

    fn clear(&self, a: Point, b: Point) -> bool {
        let steps = (((b[0] - a[0]).abs().max((b[1] - a[1]).abs()) / (self.cell / 3.0)).ceil()
            as usize)
            .max(1);
        let mut previous = None;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
            let Some(id) = self.index(p) else {
                return false;
            };
            if self.blocked[id] {
                return false;
            }
            if let Some(old) = previous
                && old % self.width != id % self.width
                && old / self.width != id / self.width
                && (self.blocked[old / self.width * self.width + id % self.width]
                    || self.blocked[id / self.width * self.width + old % self.width])
            {
                return false;
            }
            previous = Some(id);
        }
        true
    }

    /// Finds a grid route between open endpoints and removes redundant waypoints.
    ///
    /// Ordinary steps avoid blocked cells and diagonal corner cutting. Explicit
    /// pass links may cross blocked terrain and contribute their pass center to
    /// the path. The returned path starts at `a` and ends at `b`.
    ///
    /// Returns `None` for invalid or blocked endpoints, disconnected regions, an
    /// exhausted search budget, or a result exceeding 8,192 points. `None` does not
    /// distinguish an unreachable destination from a search that hit a limit.
    pub fn route(&self, a: Point, b: Point) -> Option<Vec<Point>> {
        let start = self.index(a)?;
        let end = self.index(b)?;
        if self.blocked[start] || self.blocked[end] {
            return None;
        }
        if self.connected_region(self.regions[start]) != self.connected_region(self.regions[end]) {
            return None;
        }
        if self.clear(a, b) {
            return Some(vec![a, b]);
        }
        // Octile distance uses the same 10/14 costs as cardinal/diagonal steps.
        let heuristic = |id: usize| {
            let dx = (id % self.width).abs_diff(end % self.width) as u32;
            let dy = (id / self.width).abs_diff(end / self.width) as u32;
            10 * dx.max(dy) + 4 * dx.min(dy)
        };
        let mut costs = vec![u32::MAX; self.blocked.len()];
        let mut parents = vec![usize::MAX; self.blocked.len()];
        let mut queue = BinaryHeap::new();
        // Repeated queue entries are possible when a cheaper path reaches a cell.
        // Bound total work and live queue size independently of the grid allocation.
        let max_pops = self.blocked.len().saturating_mul(4);
        let max_pushes = self.blocked.len().saturating_mul(8);
        let max_queue = self.blocked.len().saturating_mul(2);
        let mut pops = 0;
        let mut pushes = 1;
        costs[start] = 0;
        queue.push(Reverse((heuristic(start), 0, start)));
        while let Some(Reverse((_, cost, id))) = queue.pop() {
            pops += 1;
            if pops > max_pops {
                return None;
            }
            if cost != costs[id] {
                continue;
            }
            if id == end {
                let mut path = vec![b];
                let mut next = id;
                while next != start {
                    let parent = parents[next];
                    if parent == usize::MAX || parent == next || path.len() > self.blocked.len() {
                        return None;
                    }
                    if let Some(portal) = self
                        .portals
                        .get(&parent)
                        .and_then(|edges| edges.iter().find(|edge| edge.to == next))
                    {
                        path.push(portal.via);
                    }
                    next = parent;
                    path.push(self.center(next));
                }
                path.pop();
                path.push(a);
                path.reverse();
                let mut result = vec![a];
                let mut anchor = 0;
                while anchor + 1 < path.len() {
                    let mut far = anchor + 1;
                    while far + 1 < path.len() && self.clear(path[anchor], path[far + 1]) {
                        far += 1;
                    }
                    result.push(path[far]);
                    if result.len() > MAX_PATH_POINTS {
                        return None;
                    }
                    anchor = far;
                }
                return Some(result);
            }
            let x = id % self.width;
            let y = id / self.width;
            for portal in self.portals.get(&id).into_iter().flatten() {
                let candidate = cost + portal.cost;
                if candidate < costs[portal.to] {
                    costs[portal.to] = candidate;
                    parents[portal.to] = id;
                    pushes += 1;
                    if pushes > max_pushes || queue.len() >= max_queue {
                        return None;
                    }
                    queue.push(Reverse((candidate + heuristic(portal.to), candidate, portal.to)));
                }
            }
            for ny in y.saturating_sub(1)..=(y + 1).min(self.height - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(self.width - 1) {
                    let next = ny * self.width + nx;
                    if next == id || self.blocked[next] {
                        continue;
                    }
                    let diagonal = nx != x && ny != y;
                    if diagonal
                        && (self.blocked[y * self.width + nx] || self.blocked[ny * self.width + x])
                    {
                        continue;
                    }
                    let candidate = cost + if diagonal { 14 } else { 10 };
                    if candidate < costs[next] {
                        costs[next] = candidate;
                        parents[next] = id;
                        pushes += 1;
                        if pushes > max_pushes || queue.len() >= max_queue {
                            return None;
                        }
                        queue.push(Reverse((candidate + heuristic(next), candidate, next)));
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_detours_and_never_cuts_blocked_corners() {
        let mut grid = Navigation::empty([0.0, 0.0, 60.0, 60.0], 6.0).unwrap();
        for y in 0..8 {
            grid.blocked[y * 10 + 5] = true;
        }
        let path = grid.route([3.0, 3.0], [57.0, 3.0]).unwrap();
        assert!(path.len() > 2);
        assert!(path.windows(2).all(|p| grid.clear(p[0], p[1])));
        grid.blocked[85] = true;
        grid.blocked[95] = true;
        assert!(grid.route([3.0, 3.0], [57.0, 3.0]).is_none());
    }
    #[test]
    fn blocked_endpoint_requires_explicit_approach() {
        let mut grid = Navigation::empty([0.0, 0.0, 60.0, 60.0], 6.0).unwrap();
        grid.blocked[0] = true;
        assert!(grid.route([3.0, 3.0], [57.0, 3.0]).is_none());
        assert_ne!(grid.approach([3.0, 3.0]), Some([3.0, 3.0]));
    }

    #[test]
    fn site_route_uses_open_terrain_instead_of_an_isolated_gap() {
        let mut grid = Navigation::empty([0.0, 0.0, 240.0, 240.0], 6.0).unwrap();
        for y in 10..30 {
            for x in 10..30 {
                grid.blocked[y * 40 + x] = true;
            }
        }
        // An isolated one-cell gap is closer than the edge of the keep footprint.
        grid.blocked[20 * 40 + 21] = false;
        grid.index_regions();
        let start = [123.0, 123.0];
        let path = grid.route_sites(start, [231.0, 231.0]).unwrap();
        assert_ne!(path.first().copied(), grid.approach(start));
        assert!(path.windows(2).all(|p| grid.clear(p[0], p[1])));
    }

    #[test]
    fn free_endpoints_do_not_jump_across_a_closed_boundary() {
        let mut grid = Navigation::empty([0.0, 0.0, 60.0, 60.0], 6.0).unwrap();
        for y in 0..10 {
            grid.blocked[y * 10 + 5] = true;
        }
        grid.index_regions();
        assert!(grid.route_sites([27.0, 27.0], [39.0, 27.0]).is_none());
    }

    #[test]
    fn assigned_pass_is_the_only_link_through_a_closed_wall() {
        let mut grid = Navigation::empty([0.0, 0.0, 120.0, 60.0], 6.0).unwrap();
        for y in 0..10 {
            grid.blocked[y * 20 + 10] = true;
        }
        grid.index_regions();
        let pass = [63.0, 27.0];
        let start = [3.0, 27.0];
        let end = [117.0, 27.0];
        assert!(grid.route_sites(start, end).is_none());

        grid.set_passes(&[pass]).unwrap();
        let route = grid.route_sites(start, end).unwrap();
        assert!(route.contains(&pass));
        assert!(
            route
                .windows(2)
                .all(|segment| { grid.clear(segment[0], segment[1]) || segment.contains(&pass) })
        );

        grid.set_passes(&[]).unwrap();
        assert!(grid.route_sites(start, end).is_none());
    }

    #[test]
    fn walking_estimate_opens_narrow_construction_gaps_but_keeps_wide_barriers() {
        let mut grid = Navigation::empty([0.0, 0.0, 240.0, 240.0], 6.0).unwrap();
        for y in 0..40 {
            for x in 8..12 {
                grid.blocked[y * 40 + x] = true;
            }
            for x in 25..32 {
                grid.blocked[y * 40 + x] = true;
            }
        }
        grid.index_regions();
        assert!(grid.route([27.0, 123.0], [99.0, 123.0]).is_none());
        grid.reduce_building_clearance();
        grid.index_regions();
        assert!(grid.route([27.0, 123.0], [99.0, 123.0]).is_some());
        assert!(grid.route([99.0, 123.0], [231.0, 123.0]).is_none());
    }

    #[test]
    fn assigned_pass_ignores_a_sizeable_crack_inside_its_footprint() {
        let mut grid = Navigation::empty([0.0, 0.0, 240.0, 120.0], 6.0).unwrap();
        for y in 0..20 {
            for x in 15..=25 {
                grid.blocked[y * 40 + x] = true;
            }
        }
        // This 12-cell component is closer to the pass than either real zone.
        for y in 8..14 {
            for x in 18..20 {
                grid.blocked[y * 40 + x] = false;
            }
        }
        grid.index_regions();
        let pass = [123.0, 63.0];
        let start = [3.0, 63.0];
        let end = [237.0, 63.0];
        assert!(grid.route_sites(start, end).is_none());

        grid.set_passes(&[pass]).unwrap();
        let route = grid.route_sites(start, end).expect("pass should link the two map zones");
        assert!(route.contains(&pass));

        grid.set_passes(&[]).unwrap();
        assert!(grid.route_sites(start, end).is_none());
    }
}
