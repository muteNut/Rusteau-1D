//! Profile discretization and editing helpers: nodes from fixed points, regridding with interpolation of the nodal
//! values, adding and removing soil materials together with all their per-material parameters.

use crate::model::*;

fn sane_size(s: f64) -> f64 {
    if s.is_finite() && s > 0.0 {
        s
    } else {
        1.0
    }
}

/// Node depths (top to bottom) for a profile of `depth` with `n` nodes.
///
/// Every fixed point becomes a node; the surface and the bottom are always fixed points. Between two fixed points the
/// element size changes linearly from the size below the upper point to the size above the lower point, and the
/// `n - 1` elements are shared among the segments in proportion to the segment length over the mean element size,
/// with at least one per segment.
pub fn generate_depths(fixed: &[FixedPoint], depth: f64, n: usize) -> Result<Vec<f64>, String> {
    if !(depth > 0.0 && depth.is_finite()) {
        return Err("The profile depth must be positive.".into());
    }
    let eps = 1e-9 * depth;
    let mut pts: Vec<FixedPoint> = fixed
        .iter()
        .filter(|p| p.depth.is_finite() && p.depth >= -eps && p.depth <= depth + eps)
        .map(|p| FixedPoint { depth: p.depth.clamp(0.0, depth), size_above: sane_size(p.size_above), size_below: sane_size(p.size_below) })
        .collect();
    pts.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    pts.dedup_by(|later, earlier| (later.depth - earlier.depth).abs() <= eps);
    if pts.first().is_none_or(|p| p.depth > eps) {
        let s = pts.first().map(|p| p.size_above).unwrap_or(1.0);
        pts.insert(0, FixedPoint { depth: 0.0, size_above: s, size_below: s });
    }
    if pts.last().is_none_or(|p| p.depth < depth - eps) {
        let s = pts.last().map(|p| p.size_below).unwrap_or(1.0);
        pts.push(FixedPoint { depth, size_above: s, size_below: s });
    }
    pts[0].depth = 0.0;
    let last = pts.len() - 1;
    pts[last].depth = depth;
    let nseg = pts.len() - 1;
    if n < nseg + 1 {
        return Err(format!("At least {} nodes are needed for {} fixed points.", nseg + 1, pts.len()));
    }
    let n_el = n - 1;
    // ideal number of elements per segment
    let w: Vec<f64> = pts
        .windows(2)
        .map(|p| {
            let len = p[1].depth - p[0].depth;
            // segment length over the mean element size (reproduces the HYDRUS-1D distribution)
            len / (0.5 * (p[0].size_below + p[1].size_above))
        })
        .collect();
    let w_sum: f64 = w.iter().sum();
    let raw: Vec<f64> = w.iter().map(|wi| n_el as f64 * wi / w_sum.max(1e-300)).collect();
    let mut m: Vec<usize> = raw.iter().map(|r| (r.floor() as usize).max(1)).collect();
    let mut total: usize = m.iter().sum();
    while total < n_el {
        let i = (0..nseg).max_by(|&a, &b| (raw[a] - m[a] as f64).total_cmp(&(raw[b] - m[b] as f64))).unwrap();
        m[i] += 1;
        total += 1;
    }
    while total > n_el {
        let i = (0..nseg).filter(|&i| m[i] > 1).min_by(|&a, &b| (raw[a] - m[a] as f64).total_cmp(&(raw[b] - m[b] as f64))).unwrap();
        m[i] -= 1;
        total -= 1;
    }
    let mut z = vec![0.0];
    for (s, p) in pts.windows(2).enumerate() {
        let (top, bot) = (p[0].depth, p[1].depth);
        let len = bot - top;
        let k = m[s];
        let (sa, sb) = (p[0].size_below, p[1].size_above);
        // element sizes change linearly from the upper to the lower fixed point (as in HYDRUS-1D)
        let sizes: Vec<f64> = (0..k).map(|i| if k > 1 { sa + (sb - sa) * i as f64 / (k as f64 - 1.0) } else { 1.0 }).collect();
        let sum: f64 = sizes.iter().sum();
        let mut acc = top;
        for (i, e) in sizes.iter().enumerate() {
            acc += len * e / sum;
            z.push(if i + 1 == k { bot } else { acc });
        }
    }
    Ok(z)
}

fn lerp_vec(a: &[f64], b: &[f64], f: f64) -> Vec<f64> {
    (0..a.len().max(b.len()))
        .map(|j| {
            let x = a.get(j).or(b.get(j)).copied().unwrap_or(0.0);
            let y = b.get(j).copied().unwrap_or(x);
            x + (y - x) * f
        })
        .collect()
}

/// Nodal values at depth `z` of a profile `old` sorted from the top: continuous quantities are interpolated linearly,
/// material and sub-region are those of the nearer node; a node closer than `eps` is copied.
fn node_at(old: &[ProfileNode], z: f64, eps: f64) -> ProfileNode {
    let n = old.len();
    let copy = |k: usize| {
        let mut c = old[k].clone();
        c.depth = z;
        c
    };
    if z <= old[0].depth + eps {
        return copy(0);
    }
    if z >= old[n - 1].depth - eps {
        return copy(n - 1);
    }
    let i = old.partition_point(|o| o.depth <= z).saturating_sub(1).min(n - 2);
    let (a, b) = (&old[i], &old[i + 1]);
    if (z - a.depth).abs() <= eps {
        return copy(i);
    }
    if (b.depth - z).abs() <= eps {
        return copy(i + 1);
    }
    let f = ((z - a.depth) / (b.depth - a.depth)).clamp(0.0, 1.0);
    let near = if f <= 0.5 { a } else { b };
    let l = |x: f64, y: f64| x + (y - x) * f;
    ProfileNode {
        depth: z,
        h: l(a.h, b.h),
        mat: near.mat,
        layer: near.layer,
        beta: l(a.beta, b.beta),
        ah: l(a.ah, b.ah),
        ak: l(a.ak, b.ak),
        ath: l(a.ath, b.ath),
        temp: l(a.temp, b.temp),
        conc: lerp_vec(&a.conc, &b.conc, f),
        sorb: lerp_vec(&a.sorb, &b.sorb, f),
    }
}

impl Profile {
    /// Index (0-based) of the node nearest to `depth`.
    pub fn nearest_node(&self, depth: f64) -> usize {
        let mut best = 0;
        let mut bd = f64::MAX;
        for (i, n) in self.nodes.iter().enumerate() {
            let d = (n.depth - depth).abs();
            if d < bd {
                bd = d;
                best = i;
            }
        }
        best
    }

    /// Replaces the nodes by nodes at `depths` (top to bottom). Continuous nodal values are interpolated linearly,
    /// material and sub-region numbers come from the nearer old node, values beyond the old bottom are those of the
    /// bottom node. Observation nodes keep their depth (they move to the nearest new node).
    pub fn regrid(&mut self, depths: &[f64]) {
        let old = std::mem::take(&mut self.nodes);
        if old.is_empty() {
            self.nodes = depths.iter().map(|&z| ProfileNode::new(z, -100.0, 1)).collect();
            return;
        }
        let obs_depths: Vec<f64> = self.observation_nodes.iter().filter_map(|&k| old.get(k.wrapping_sub(1)).map(|n| n.depth)).collect();
        let scale = old.iter().map(|n| n.depth.abs()).chain(depths.iter().map(|z| z.abs())).fold(0.0f64, f64::max);
        let eps = 1e-9 * scale.max(1e-12);
        self.nodes = depths.iter().map(|&z| node_at(&old, z, eps)).collect();
        let mut obs: Vec<usize> = vec![];
        for d in obs_depths {
            let k = self.nearest_node(d) + 1;
            if !obs.contains(&k) {
                obs.push(k);
            }
        }
        obs.sort_unstable();
        self.observation_nodes = obs;
    }

    /// The fixed points, or the surface and the bottom when none are stored.
    pub fn fixed_points_or_ends(&self) -> Vec<FixedPoint> {
        if self.fixed_points.is_empty() {
            vec![FixedPoint::new(0.0), FixedPoint::new(self.depth())]
        } else {
            self.fixed_points.clone()
        }
    }
}

impl Project {
    /// Appends a copy of material `k` (0-based) with all its per-material parameters (water flow, tables, dual
    /// permeability, heat, solute, root uptake) and returns the index of the new material.
    pub fn add_material_copy(&mut self, k: usize) -> usize {
        let nm = self.water.materials.len();
        let k = k.min(nm.saturating_sub(1));
        let ns = self.solute.species.len();
        let mut m = self.water.materials.get(k).cloned().unwrap_or_default();
        m.name = format!("Material {}", nm + 1);
        self.water.materials.push(m);
        if let Some(t) = self.water.tabs.as_mut() {
            if let Some(x) = t.get(k).cloned() {
                t.push(x);
            }
        }
        if let Some(x) = self.water.dual_perm.materials.get(k).cloned() {
            self.water.dual_perm.materials.push(x);
        }
        if let Some(x) = self.heat.materials.get(k).cloned() {
            self.heat.materials.push(x);
        }
        if let Some(x) = self.solute.materials.get(k).cloned() {
            self.solute.materials.push(x);
        }
        for sp in self.solute.species.iter_mut() {
            if let Some(x) = sp.per_material.get(k).cloned() {
                sp.per_material.push(x);
            }
        }
        if let RootStress::Feddes { p_optm, .. } = &mut self.root.stress {
            if let Some(x) = p_optm.get(k).copied() {
                p_optm.push(x);
            }
        }
        if ns > 0 && self.solute.moist_tables.len() == nm * ns {
            let chunk: Vec<MoistDepTable> = self.solute.moist_tables[k * ns..(k + 1) * ns].to_vec();
            self.solute.moist_tables.extend(chunk);
        }
        nm
    }

    /// Removes material `k` (0-based) with its per-material parameters. Nodes of that material get the material
    /// before it (the new first material when `k` = 0); the numbers of the following materials move up by one.
    pub fn remove_material(&mut self, k: usize) {
        let nm = self.water.materials.len();
        if nm <= 1 || k >= nm {
            return;
        }
        let ns = self.solute.species.len();
        self.water.materials.remove(k);
        if let Some(t) = self.water.tabs.as_mut() {
            if k < t.len() {
                t.remove(k);
            }
        }
        if k < self.water.dual_perm.materials.len() {
            self.water.dual_perm.materials.remove(k);
        }
        if k < self.heat.materials.len() {
            self.heat.materials.remove(k);
        }
        if k < self.solute.materials.len() {
            self.solute.materials.remove(k);
        }
        for sp in self.solute.species.iter_mut() {
            if k < sp.per_material.len() {
                sp.per_material.remove(k);
            }
        }
        if let RootStress::Feddes { p_optm, .. } = &mut self.root.stress {
            if k < p_optm.len() {
                p_optm.remove(k);
            }
        }
        if ns > 0 && self.solute.moist_tables.len() == nm * ns {
            self.solute.moist_tables.drain(k * ns..(k + 1) * ns);
        }
        let r = k + 1;
        for n in self.profile.nodes.iter_mut() {
            if n.mat == r {
                n.mat = (r - 1).max(1);
            } else if n.mat > r {
                n.mat -= 1;
            }
        }
    }
}
