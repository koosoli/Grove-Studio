//! The game LOD ladder: four detail levels a game engine can afford for a whole forest.
//!
//! The editor's own LODs only thin the leaves. A forest draws hundreds of trees at once, so the
//! ladder here also simplifies the wood (meshoptimizer), turns leaves into alpha-cut cards and,
//! further away, gathers them into clustered spray cards baked from the tree's own leaves:
//!
//! | level    | budget (triangles) | wood                         | leaves                          |
//! |----------|--------------------|------------------------------|---------------------------------|
//! | near     | <= 150 000         | simplified, every branch     | folded alpha cards (4 triangles)|
//! | mid      | 20 000 - 40 000    | simplified, twigs dropped    | flat alpha cards (2 triangles)  |
//! | far      | 5 000 - 8 000      | trunk and limbs              | clustered spray cards           |
//! | very far | 1 000 - 2 000      | trunk and main limbs         | a few large spray cards         |
//!
//! Conifers keep their needle fans at near and use spray cards from mid on, since a fan is a
//! single triangle already. Budgets count single-sided triangles (as `--inspect` does); an
//! exporter that writes two-sided foliage writes the foliage twice.
//!
//! Every level shares the ground origin, the vertex normals of the source geometry and the
//! bark UVs, so wind (by model height), lighting and bark relief behave as on the full tree.
use crate::tree::{Part, Settings, V};
use macroquad::prelude::{vec2, Vec2, Vec3};
use solstice_procgen::foliage::leaf::LeafKind;
use std::collections::HashMap;

/// Leaf atlas regions in V (the texture is ATLAS x ATLAS). U keeps Solstice's four variant
/// columns in the leaf regions.
pub const ATLAS: u32 = 512;
/// Opaque leaf columns: pieces whose outline is their geometry (needle fans).
pub const OPAQUE: (f32, f32) = (0.02, 0.23);
/// Alpha-cut leaf columns: the leaf's outline in alpha, for card leaves.
pub const CUTOUT: (f32, f32) = (0.27, 0.48);
/// Two spray cards side by side, baked from the tree's own leaves.
pub const SPRAY: (f32, f32) = (0.52, 0.98);
pub const SPRAY_U: [(f32, f32); 2] = [(0.01, 0.49), (0.51, 0.99)];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LeafStyle {
    /// A folded alpha card: four triangles.
    Folded,
    /// A flat alpha card: two triangles.
    Flat,
    /// Spray cards over clusters of leaves.
    Clusters,
    /// The mid level's spray cards plus thinned source pieces (conifers up close).
    PiecesOverSprays,
    /// The source leaf pieces themselves, thinned with their area kept, opaque: their outline is
    /// their geometry, so no alpha test. For an engine that simplifies the whole tree per cluster
    /// (OpenPoseidon GROVE-OPAQUE, `--opaque-leaves`), as Solstice draws its own trees.
    Opaque,
}

#[derive(Clone, Copy, Debug)]
pub struct Rung {
    pub name: &'static str,
    /// Single-sided triangle budget of the whole level.
    pub budget: usize,
    /// Most of the budget the wood may take.
    pub wood: usize,
    /// meshoptimizer target error, relative to the tree's extent.
    pub wood_error: f32,
    /// Branch shells shorter than this fraction of the tree's height are dropped.
    pub min_branch: f32,
    pub broadleaf: LeafStyle,
    pub needles: LeafStyle,
    /// The engine resolution of the level when the model is used on its own.
    pub resolution: f32,
}

pub const LADDER: [Rung; 4] = [
    Rung {
        name: "near",
        budget: 150_000,
        wood: 40_000,
        wood_error: 0.002,
        min_branch: 0.0,
        broadleaf: LeafStyle::Folded,
        needles: LeafStyle::PiecesOverSprays,
        resolution: 0.5,
    },
    Rung {
        name: "mid",
        budget: 30_000,
        wood: 7_000,
        wood_error: 0.01,
        min_branch: 0.02,
        broadleaf: LeafStyle::Flat,
        needles: LeafStyle::Clusters,
        resolution: 1.0,
    },
    Rung {
        name: "far",
        budget: 6_500,
        wood: 1_500,
        wood_error: 0.04,
        min_branch: 0.06,
        broadleaf: LeafStyle::Clusters,
        needles: LeafStyle::Clusters,
        resolution: 3.0,
    },
    Rung {
        name: "very far",
        budget: 1_500,
        wood: 350,
        wood_error: 0.15,
        min_branch: 0.18,
        broadleaf: LeafStyle::Clusters,
        needles: LeafStyle::Clusters,
        resolution: 8.0,
    },
];

pub struct Level {
    pub rung: Rung,
    pub bark: Part,
    /// UVs already in the game atlas (see ATLAS, OPAQUE, CUTOUT, SPRAY).
    pub leaves: Part,
}

impl Level {
    pub fn triangles(&self) -> usize {
        (self.bark.i.len() + self.leaves.i.len()) / 3
    }
    pub fn vertices(&self) -> usize {
        self.bark.v.len() + self.leaves.v.len()
    }
}

/// One source leaf, read back from its mesh piece.
#[derive(Clone, Copy)]
struct Leaf {
    base: Vec3,
    tip: Vec3,
    side: Vec3,
    face: Vec3,
    width: f32,
    variant: f32,
    n_base: Vec3,
    n_mid: Vec3,
    n_tip: Vec3,
    color: [f32; 4],
}

impl Leaf {
    fn center(&self) -> Vec3 {
        (self.base + self.tip) * 0.5
    }
    fn length(&self) -> f32 {
        self.base.distance(self.tip)
    }
}

/// A finished tree's game ladder.
pub struct Ladder {
    pub levels: Vec<Level>,
    /// The leaf atlas: ATLAS x ATLAS RGBA, sRGB colour with the cutout alpha.
    pub atlas: Vec<u8>,
}

/// Clip alpha cards to a padded convex hull of the base atlas alpha. Opaque pieces are kept.
/// Clipping each source triangle separately preserves folds and interpolates its attributes.
/// Coarse mipmaps, compression and vertex-driven wind can still change the image.
pub fn trim_cards(ladder: &mut Ladder) {
    let regions: Vec<_> = (0..4).map(|i| (i as f32 / 4., (i + 1) as f32 / 4., CUTOUT))
        .chain(SPRAY_U.into_iter().map(|(a,b)| (a,b,SPRAY))).collect();
    let hulls: Vec<_> = regions.iter().map(|&(u0,u1,(v0,v1))| {
        let mut points = Vec::new();
        for y in (v0 * ATLAS as f32) as u32..((v1 * ATLAS as f32).ceil() as u32).min(ATLAS) {
            for x in (u0 * ATLAS as f32) as u32..((u1 * ATLAS as f32).ceil() as u32).min(ATLAS) {
                if ladder.atlas[((y * ATLAS + x) * 4 + 3) as usize] != 0 {
                    // Four texels cover base-level filtering and most BC3 block spread.
                    for (dx,dy) in [(-4.,-4.), (5.,-4.), (5.,5.), (-4.,5.)] {
                        points.push(vec2((x as f32 + dx) / ATLAS as f32, (y as f32 + dy) / ATLAS as f32));
                    }
                }
            }
        }
        points.sort_by(|a,b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        points.dedup();
        fn cross(a: Vec2,b: Vec2,c: Vec2) -> f32 { (b.x-a.x)*(c.y-a.y)-(b.y-a.y)*(c.x-a.x) }
        let mut hull = Vec::new();
        for &p in &points {
            while hull.len() >= 2 && cross(hull[hull.len()-2], hull[hull.len()-1], p) <= 0. { hull.pop(); }
            hull.push(p);
        }
        let lower = hull.len();
        for &p in points.iter().rev().skip(1) {
            while hull.len() > lower && cross(hull[hull.len()-2], hull[hull.len()-1], p) <= 0. { hull.pop(); }
            hull.push(p);
        }
        if hull.len() > 1 { hull.pop(); }
        if hull.len() < 3 { return Vec::new(); }
        // Intersect eight support half-planes around the hull. This contains every alpha texel
        // while bounding tessellation regardless of how irregular the baked texture is.
        let mut outline = vec![vec2(-1.,-1.), vec2(2.,-1.), vec2(2.,2.), vec2(-1.,2.)];
        for k in 0..8 {
            let angle = k as f32 * std::f32::consts::FRAC_PI_4;
            let normal = vec2(angle.cos(), angle.sin());
            let limit = hull.iter().map(|p| p.dot(normal)).fold(f32::NEG_INFINITY, f32::max);
            let input = std::mem::take(&mut outline);
            let mut previous = *input.last().unwrap();
            let mut dp = limit - previous.dot(normal);
            for current in input {
                let dc = limit - current.dot(normal);
                if (dp >= 0.) != (dc >= 0.) { outline.push(previous.lerp(current, dp / (dp-dc))); }
                if dc >= 0. { outline.push(current); }
                previous = current; dp = dc;
            }
        }
        outline
    }).collect();
    for level in ladder.levels.iter_mut().skip(1) {
        let old = std::mem::take(&mut level.leaves);
        let mut result = Part::default();
        for tri in old.i.chunks_exact(3) {
            let vertices = [old.v[tri[0] as usize], old.v[tri[1] as usize], old.v[tri[2] as usize]];
            let uv = (vertices[0].uv + vertices[1].uv + vertices[2].uv) / 3.;
            let region = if (CUTOUT.0..=CUTOUT.1).contains(&uv.y) {
                Some(((uv.x * 4.) as usize).min(3))
            } else if (SPRAY.0..=SPRAY.1).contains(&uv.y) {
                Some(4 + usize::from(uv.x >= 0.5))
            } else { None };
            let mut polygon = vertices.to_vec();
            if let Some(hull) = region.map(|i| &hulls[i]).filter(|h| h.len() >= 3) {
                for edge in 0..hull.len() {
                    let a = hull[edge]; let b = hull[(edge + 1) % hull.len()];
                    let side = |p: Vec2| (b.x-a.x)*(p.y-a.y)-(b.y-a.y)*(p.x-a.x);
                    let input = std::mem::take(&mut polygon);
                    if input.is_empty() { break; }
                    let mut previous = *input.last().unwrap();
                    let mut dp = side(previous.uv);
                    for current in input {
                        let dc = side(current.uv);
                        if (dp >= 0.) != (dc >= 0.) {
                            let t = dp / (dp - dc);
                            polygon.push(V { p: previous.p.lerp(current.p,t), n: previous.n.lerp(current.n,t),
                                uv: previous.uv.lerp(current.uv,t), c: std::array::from_fn(|i| previous.c[i] + (current.c[i]-previous.c[i])*t) });
                        }
                        if dc >= 0. { polygon.push(current); }
                        previous = current; dp = dc;
                    }
                }
                polygon.dedup_by(|a,b| a.uv.distance_squared(b.uv) < 1e-14);
                if polygon.len() > 1 && polygon[0].uv.distance_squared(polygon[polygon.len()-1].uv) < 1e-14 { polygon.pop(); }
            }
            if polygon.len() >= 3 {
                let base = result.v.len() as u32;
                for i in 1..polygon.len()-1 {
                    let a = polygon[i].uv - polygon[0].uv; let b = polygon[i+1].uv - polygon[0].uv;
                    if region.is_none() || (a.x*b.y-a.y*b.x).abs() > 1e-12 {
                        result.i.extend([base, base+i as u32, base+i as u32+1]);
                    }
                }
                result.v.extend(polygon);
            }
        }
        level.leaves = result;
    }
}

pub fn ladder(s: &Settings) -> Ladder {
    ladder_with(s, &LADDER)
}

/// The rungs with other triangle budgets (`--budgets near,mid,far,veryfar`): each rung's wood
/// share scales with it. A dense forest draws thousands of trees past 30 m, so its far rungs
/// must be far cheaper than a lone tree's (2026-11-08: the default 6.5k/1.5k cost 3x the
/// engine's own trees there).
pub fn rungs_with_budgets(budgets: &[usize]) -> Result<[Rung; 4], String> {
    if budgets.len() != 4 || budgets.windows(2).any(|w| w[1] >= w[0]) || budgets[3] < 200 {
        return Err("--budgets wants four shrinking triangle counts, the last at least 200".into());
    }
    let mut rungs = LADDER;
    for (rung, &b) in rungs.iter_mut().zip(budgets) {
        rung.wood = (rung.wood as f64 * b as f64 / rung.budget as f64).round().max(60.) as usize;
        rung.budget = b;
    }
    Ok(rungs)
}

/// The two-level ladder of `--opaque-leaves`: a near level of opaque leaf pieces that the engine
/// reduces per cluster with distance, and the very far spray-card level for shadows and the far
/// forest hand-over. `budgets` = (near, very far) single-sided triangles.
pub fn opaque_rungs(budgets: Option<&[usize]>) -> Result<[Rung; 2], String> {
    let mut near = LADDER[0];
    near.name = "near (opaque)";
    near.broadleaf = LeafStyle::Opaque;
    near.needles = LeafStyle::Opaque;
    let mut far = LADDER[3];
    if let Some(b) = budgets {
        if b.len() != 2 || b[1] >= b[0] || b[1] < 200 {
            return Err("--budgets with --opaque-leaves wants two shrinking triangle counts (near, very far)".into());
        }
        near.wood = (near.wood as f64 * b[0] as f64 / near.budget as f64).round().max(60.) as usize;
        near.budget = b[0];
        far.wood = (far.wood as f64 * b[1] as f64 / far.budget as f64).round().max(60.) as usize;
        far.budget = b[1];
    }
    Ok([near, far])
}

pub fn ladder_with(s: &Settings, rungs: &[Rung]) -> Ladder {
    let mut full = crate::tree::generate(s, 0);
    // The origin is the trunk's foot, not the lowest point of the wood: a fir's lowest whorls
    // droop below where its trunk starts, and the far levels drop them.
    let foot = trunk_foot(&full.bark);
    for v in full.bark.v.iter_mut().chain(&mut full.leaves.v) {
        v.p.y -= foot;
    }
    let kind = crate::reference::leaf_kind(s).unwrap_or(LeafKind::Broadleaf);
    let needles = kind == LeafKind::Needles;
    let colors = crate::preview::leaf_rgba(s);
    let height = full.bounds().1.y.max(1.0);
    let pieces = if needles { 3 } else { 8 };
    let leaves = read_leaves(&full.leaves, kind);
    let crown = if full.leaves.v.is_empty() {
        Vec3::Y * height * 0.7
    } else {
        full.leaves.v.iter().map(|v| v.p).sum::<Vec3>() / full.leaves.v.len() as f32
    };
    let mut atlas = leaf_atlas(&colors, kind);
    // three crossed broadleaf cards over-cover a cluster's leaves, so they shrink toward the
    // silhouette of the card leaves (checked by eye, 2026-11-08); needle sprays are open
    let card_scale = if needles { 1.0 } else { 0.65 };
    // The sprays are baked once, from clusters of the size the far level uses, and scaled for
    // every level that draws them.
    let bake_budget = if rungs.len() > 2 { rungs[2].budget } else { LADDER[2].budget };
    let bake = cluster(&leaves, bake_budget * 3 / 4 / 6, card_scale);
    if !bake.is_empty() {
        bake_sprays(&mut atlas, &colors, &full.leaves, pieces, &leaves, &bake);
    }
    let mut levels = Vec::new();
    for &rung in rungs {
        let bark = simplify_wood(&full.bark, &rung, height);
        let left = rung.budget.saturating_sub(bark.i.len() / 3);
        let style = if needles { rung.needles } else { rung.broadleaf };
        let foliage = match style {
            LeafStyle::Folded => cards(&leaves, left, true),
            LeafStyle::Flat => cards(&leaves, left, false),
            LeafStyle::Clusters => spray_cards(&leaves, &cluster(&leaves, left / 6, card_scale), crown),
            LeafStyle::Opaque => opaque_pieces(&full.leaves, pieces, left),
            LeafStyle::PiecesOverSprays => {
                // the mid level's spray cards as the body of the crown, and needle fans in the
                // rest of the budget for the close-up detail: near hands over to mid without
                // the crown thinning out
                let body = rungs.get(1).map_or(LADDER[1].budget, |r| r.budget) * 3 / 4;
                let mut part = spray_cards(&leaves, &cluster(&leaves, body / 6, card_scale), crown);
                let fans = thin_pieces(&full.leaves, pieces, left.saturating_sub(body));
                let first = part.v.len() as u32;
                part.v.extend(fans.v);
                part.i.extend(fans.i.iter().map(|i| i + first));
                part
            }
        };
        levels.push(Level { rung, bark, leaves: foliage });
    }
    Ladder { levels, atlas }
}

/// Inspection statistics for a ladder.
pub fn stats(l: &Ladder) -> serde_json::Value {
    serde_json::json!(l
        .levels
        .iter()
        .map(|level| serde_json::json!({
            "level": level.rung.name,
            "resolution": level.rung.resolution,
            "triangles": level.triangles(),
            "wood_triangles": level.bark.i.len() / 3,
            "leaf_triangles": level.leaves.i.len() / 3,
            "two_sided_faces": level.bark.i.len() / 3 + level.leaves.i.len() * 2 / 3,
            "vertices": level.vertices(),
            "estimated_vertex_bytes": level.vertices() * 72,
        }))
        .collect::<Vec<_>>())
}

fn read_leaves(part: &Part, kind: LeafKind) -> Vec<Leaf> {
    let mut out = Vec::new();
    if kind == LeafKind::Needles {
        for piece in part.v.chunks_exact(3) {
            // A fan: two root corners and a tip. Its card spans the fan's width at the root.
            let base = (piece[0].p + piece[1].p) * 0.5;
            let tip = piece[2].p;
            let side = (piece[1].p - piece[0].p).normalize_or_zero();
            let axis = (tip - base).normalize_or_zero();
            out.push(Leaf {
                base,
                tip,
                side,
                face: side.cross(axis).normalize_or_zero(),
                width: piece[0].p.distance(piece[1].p),
                variant: ((piece[2].uv.x * 4.) as usize).min(3) as f32,
                n_base: piece[0].n,
                n_mid: piece[2].n,
                n_tip: piece[2].n,
                color: piece[0].c,
            });
        }
        return out;
    }
    let (h0, h1) = (kind.half_width(0.32), kind.half_width(0.68));
    for piece in part.v.chunks_exact(8) {
        let (base, tip) = (piece[0].p, piece[7].p);
        let (row0, row1) = (piece[3].p - piece[1].p, piece[6].p - piece[4].p);
        let side = (row0 + row1).normalize_or_zero();
        let axis = (tip - base).normalize_or_zero();
        let width = (row0.length() + row1.length()) / (2. * (h0 + h1)).max(1e-6);
        out.push(Leaf {
            base,
            tip,
            side,
            face: side.cross(axis).normalize_or_zero(),
            width,
            variant: ((piece[0].uv.x * 4.) as usize).min(3) as f32,
            n_base: piece[0].n,
            n_mid: piece[2].n,
            n_tip: piece[7].n,
            color: piece[0].c,
        });
    }
    out
}

/// Every n-th of `total` items so that the kept ones fit `budget` units of `each`, and how much
/// to grow each kept one to keep most of the covered area.
fn stride(total: usize, each: usize, budget: usize) -> (usize, f32) {
    let n = (total * each).div_ceil(budget.max(each)).max(1);
    // area-preserving: the kept pieces cover what all of them covered, so a thinned crown keeps
    // its silhouette and density at the distance it is drawn (coverage, not triangle count, is
    // what reads at 45 m)
    (n, (n as f32).sqrt())
}

fn uv_in(region: (f32, f32), v: f32) -> f32 {
    region.0 + (region.1 - region.0) * v.clamp(0., 1.)
}

fn column_u(variant: f32, across: f32) -> f32 {
    (variant + 0.5 + across * 0.47) / 4.
}

/// The source pieces (needle fans), every n-th, grown about their centres; UVs remapped into the
/// opaque region.
fn thin_pieces(part: &Part, verts: usize, budget: usize) -> Part {
    let count = part.v.len() / verts;
    let (n, grow) = stride(count, 1, budget);
    let mut out = Part::default();
    for (k, piece) in part.v.chunks_exact(verts).enumerate() {
        if k % n != 0 {
            continue;
        }
        let center = piece.iter().map(|v| v.p).sum::<Vec3>() / verts as f32;
        let first = out.v.len() as u32;
        for v in piece {
            let mut v = *v;
            v.p = center + (v.p - center) * grow;
            v.uv = vec2(v.uv.x, uv_in(OPAQUE, v.uv.y));
            out.v.push(v);
        }
        // Keep each fan's winding: counter-clockwise seen from its face (alternate sides of a
        // shoot wind the other way in the source).
        let (a, b, c) = (piece[0].p, piece[1].p, piece[2].p);
        let ccw = (b - a).cross(c - a).dot(piece[0].n) >= 0.;
        out.i
            .extend(if ccw { [first, first + 1, first + 2] } else { [first, first + 2, first + 1] });
    }
    out
}

/// The source leaf pieces (`verts` vertices each, their own triangles), every n-th kept and grown
/// about its centre so the kept ones cover what all of them covered; UVs into the opaque band.
fn opaque_pieces(part: &Part, verts: usize, budget: usize) -> Part {
    let count = part.v.len() / verts;
    if count == 0 {
        return Part::default();
    }
    let mut tris: Vec<Vec<[u32; 3]>> = vec![Vec::new(); count];
    for t in part.i.chunks_exact(3) {
        let k = t[0] as usize / verts;
        if k < count && t.iter().all(|&i| i as usize / verts == k) {
            tris[k].push([t[0], t[1], t[2]]);
        }
    }
    let each = (part.i.len() / 3).div_ceil(count).max(1);
    let (n, grow) = stride(count, each, budget);
    let mut out = Part::default();
    for (k, piece) in part.v.chunks_exact(verts).enumerate() {
        if k % n != 0 || tris[k].is_empty() {
            continue;
        }
        let center = piece.iter().map(|v| v.p).sum::<Vec3>() / verts as f32;
        let first = out.v.len() as u32;
        for v in piece {
            let mut v = *v;
            v.p = center + (v.p - center) * grow;
            v.uv = vec2(v.uv.x, uv_in(OPAQUE, v.uv.y));
            out.v.push(v);
        }
        let base = (k * verts) as u32;
        for t in &tris[k] {
            out.i.extend(t.iter().map(|i| i - base + first));
        }
    }
    out
}

/// Leaves as alpha-cut cards spanning the leaf's length and full width: four triangles with a
/// fold along the midrib (`folded`), or two flat ones.
fn cards(leaves: &[Leaf], budget: usize, folded: bool) -> Part {
    let each = if folded { 4 } else { 2 };
    let (n, grow) = stride(leaves.len(), each, budget);
    let mut out = Part::default();
    for leaf in leaves.iter().step_by(n) {
        let center = leaf.center();
        let scale = |p: Vec3| center + (p - center) * grow;
        let half = leaf.side * (leaf.width * 0.5 * grow);
        let (base, tip) = (scale(leaf.base), scale(leaf.tip));
        let lift = leaf.face * (leaf.width * 0.12 * grow);
        let vertex = |p: Vec3, n: Vec3, across: f32, along: f32| V {
            p,
            n,
            c: leaf.color,
            uv: vec2(column_u(leaf.variant, across), uv_in(CUTOUT, along)),
        };
        let first = out.v.len() as u32;
        if folded {
            // edges raised above the midrib, as the source leaf folds
            out.v.extend([
                vertex(base - half + lift, leaf.n_base, -1., 0.),
                vertex(base, leaf.n_base, 0., 0.),
                vertex(base + half + lift, leaf.n_base, 1., 0.),
                vertex(tip - half + lift, leaf.n_tip, -1., 1.),
                vertex(tip, leaf.n_tip, 0., 1.),
                vertex(tip + half + lift, leaf.n_tip, 1., 1.),
            ]);
            let [bl, bc, br, tl, tc, tr] = std::array::from_fn(|k| first + k as u32);
            out.i.extend([bl, bc, tc, bl, tc, tl, bc, br, tr, bc, tr, tc]);
        } else {
            out.v.extend([
                vertex(base - half, leaf.n_base, -1., 0.),
                vertex(base + half, leaf.n_base, 1., 0.),
                vertex(tip - half, leaf.n_mid, -1., 1.),
                vertex(tip + half, leaf.n_mid, 1., 1.),
            ]);
            let [bl, br, tl, tr] = std::array::from_fn(|k| first + k as u32);
            out.i.extend([bl, br, tr, bl, tr, tl]);
        }
    }
    out
}

/// A group of nearby leaves drawn as one spray.
struct Cluster {
    center: Vec3,
    radius: f32,
    normal: Vec3,
    members: Vec<usize>,
}

/// Groups the leaves on a grid whose cell is chosen so that about `target` clusters result.
/// `card_scale` sizes the cards against the cluster's reach (see `ladder`).
fn cluster(leaves: &[Leaf], target: usize, card_scale: f32) -> Vec<Cluster> {
    if leaves.is_empty() || target == 0 {
        return Vec::new();
    }
    let group = |cell: f32| {
        let mut cells: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
        for (k, leaf) in leaves.iter().enumerate() {
            let c = leaf.center() / cell;
            cells
                .entry((c.x.floor() as i32, c.y.floor() as i32, c.z.floor() as i32))
                .or_default()
                .push(k);
        }
        cells
    };
    let (mut lo, mut hi) = (0.02_f32, 20.0_f32);
    for _ in 0..24 {
        let mid = (lo * hi).sqrt();
        if group(mid).len() > target {
            lo = mid
        } else {
            hi = mid
        }
    }
    let cell = hi;
    let mut cells: Vec<_> = group(cell).into_iter().collect();
    cells.sort_by_key(|(key, _)| *key);
    cells
        .into_iter()
        .map(|(_, members)| {
            let center =
                members.iter().map(|&k| leaves[k].center()).sum::<Vec3>() / members.len() as f32;
            let reach = members
                .iter()
                .map(|&k| leaves[k].center().distance(center) + leaves[k].length() * 0.5)
                .fold(0., f32::max);
            let normal = members
                .iter()
                .map(|&k| leaves[k].face)
                .sum::<Vec3>()
                .normalize_or(Vec3::Y);
            Cluster {
                center,
                radius: reach.min(cell * 0.8).max(cell * 0.35) * card_scale,
                normal,
                members,
            }
        })
        .collect()
}

fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

fn orthonormal(n: Vec3) -> (Vec3, Vec3) {
    let a = if n.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
    let t = n.cross(a).normalize();
    (t, n.cross(t).normalize())
}

/// Three crossed spray cards per cluster: facing out of the crown, across it and up. Their
/// normals lean out from the crown's centre, as the source leaves' do.
fn spray_cards(leaves: &[Leaf], clusters: &[Cluster], crown: Vec3) -> Part {
    let mut out = Part::default();
    for (k, c) in clusters.iter().enumerate() {
        let h = hash(k as u32 * 3 + 1);
        let mut out_dir = Vec3::new(c.center.x, 0., c.center.z);
        if out_dir.length_squared() < 1e-4 {
            out_dir = Vec3::X;
        }
        let out_dir = out_dir.normalize();
        let across = Vec3::Y.cross(out_dir).normalize();
        let up = (Vec3::Y * 0.8 + c.normal * 0.2).normalize_or(Vec3::Y);
        let lit = (c.center - crown).normalize_or(Vec3::Y);
        let color = leaves[c.members[0]].color;
        let spray = SPRAY_U[(h & 1) as usize];
        // A card never hangs below its cluster's lowest leaf: big far cards otherwise drop a
        // curtain of foliage to head height under the crowns (2026-11-08, in game).
        let floor = c
            .members
            .iter()
            .map(|&k| leaves[k].base.y.min(leaves[k].tip.y))
            .fold(f32::MAX, f32::min);
        for (j, normal) in [out_dir, across, up].into_iter().enumerate() {
            let (t0, b0) = orthonormal(normal);
            let angle = (hash(h ^ j as u32) & 0xffff) as f32 / 65535. * std::f32::consts::TAU;
            let (sin, cos) = angle.sin_cos();
            let (t, b) = (t0 * cos + b0 * sin, b0 * cos - t0 * sin);
            let n = (normal * 0.4 + lit * 0.6).normalize_or(normal);
            let first = out.v.len() as u32;
            for (x, y) in [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)] {
                let mut p = c.center + (t * x + b * y) * c.radius;
                p.y = p.y.max(floor);
                out.v.push(V {
                    p,
                    n,
                    c: color,
                    uv: vec2(
                        spray.0 + (spray.1 - spray.0) * (x * 0.5 + 0.5),
                        uv_in(SPRAY, y * 0.5 + 0.5),
                    ),
                });
            }
            // counter-clockwise seen from `normal`: t x b = normal
            out.i.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
        }
    }
    out
}

/// Simplifies the wood to a level's budget: drops the branch shells that are too short to see,
/// then collapses edges with meshoptimizer (seams and attribute borders handled by it).
pub fn simplify_wood(bark: &Part, rung: &Rung, height: f32) -> Part {
    let mut indices = bark.i.clone();
    if rung.min_branch > 0. {
        indices = drop_small_components(bark, rung.min_branch * height);
    }
    let positions: Vec<[f32; 3]> = bark.v.iter().map(|v| v.p.to_array()).collect();
    let adapter =
        meshopt::VertexDataAdapter::new(meshopt::typed_to_bytes(&positions), 12, 0).unwrap();
    // The root ring stays where it is: the tree must keep standing on its origin.
    let lowest = bark.v.iter().map(|v| v.p.y).fold(f32::MAX, f32::min);
    let lock: Vec<bool> = bark
        .v
        .iter()
        .map(|v| v.p.y < lowest + 0.02 * height)
        .collect();
    let target = (rung.wood * 3).min(indices.len());
    if target < indices.len() || rung.wood_error > 0. {
        indices = meshopt::simplify_with_locks(
            &indices,
            &adapter,
            &lock,
            target,
            rung.wood_error,
            meshopt::SimplifyOptions::None,
            None,
        );
        if indices.len() > rung.wood * 3 {
            indices = meshopt::simplify_sloppy_with_locks(
                &indices,
                &adapter,
                &lock,
                rung.wood * 3,
                1.0,
                None,
            );
        }
    }
    compact(bark, &indices)
}

/// Height of the lowest point of the tallest shell (the trunk).
fn trunk_foot(part: &Part) -> f32 {
    let mut parent: Vec<u32> = (0..part.v.len() as u32).collect();
    fn find(parent: &mut [u32], mut x: u32) -> u32 {
        while parent[x as usize] != x {
            parent[x as usize] = parent[parent[x as usize] as usize];
            x = parent[x as usize];
        }
        x
    }
    for t in part.i.chunks_exact(3) {
        let a = find(&mut parent, t[0]);
        for &other in &t[1..] {
            let b = find(&mut parent, other);
            if a != b {
                parent[b as usize] = a;
            }
        }
    }
    let mut span: HashMap<u32, (f32, f32)> = HashMap::new();
    for k in 0..part.v.len() as u32 {
        let root = find(&mut parent, k);
        let y = part.v[k as usize].p.y;
        let e = span.entry(root).or_insert((y, y));
        e.0 = e.0.min(y);
        e.1 = e.1.max(y);
    }
    span.values()
        .max_by(|a, b| (a.1 - a.0).total_cmp(&(b.1 - b.0)))
        .map(|s| s.0)
        .unwrap_or(0.)
}

/// The triangles of the connected shells whose extent reaches `min_length`.
fn drop_small_components(part: &Part, min_length: f32) -> Vec<u32> {
    let mut parent: Vec<u32> = (0..part.v.len() as u32).collect();
    fn find(parent: &mut [u32], mut x: u32) -> u32 {
        while parent[x as usize] != x {
            parent[x as usize] = parent[parent[x as usize] as usize];
            x = parent[x as usize];
        }
        x
    }
    for t in part.i.chunks_exact(3) {
        let a = find(&mut parent, t[0]);
        for &other in &t[1..] {
            let b = find(&mut parent, other);
            if a != b {
                parent[b as usize] = a;
            }
        }
    }
    let mut bounds: HashMap<u32, (Vec3, Vec3)> = HashMap::new();
    for k in 0..part.v.len() as u32 {
        let root = find(&mut parent, k);
        let p = part.v[k as usize].p;
        let e = bounds.entry(root).or_insert((p, p));
        e.0 = e.0.min(p);
        e.1 = e.1.max(p);
    }
    let keep: HashMap<u32, bool> = bounds
        .iter()
        .map(|(root, (lo, hi))| (*root, (*hi - *lo).length() >= min_length))
        .collect();
    let mut out = Vec::with_capacity(part.i.len());
    for t in part.i.chunks_exact(3) {
        if keep[&find(&mut parent, t[0])] {
            out.extend_from_slice(t);
        }
    }
    // Never lose the trunk: if everything was too small, keep the largest shell.
    if out.is_empty() && !part.i.is_empty() {
        let largest = bounds
            .iter()
            .max_by(|a, b| (a.1 .1 - a.1 .0).length().total_cmp(&(b.1 .1 - b.1 .0).length()))
            .map(|(root, _)| *root)
            .unwrap();
        for t in part.i.chunks_exact(3) {
            if find(&mut parent, t[0]) == largest {
                out.extend_from_slice(t);
            }
        }
    }
    out
}

/// A part holding only the vertices `indices` use, in first-use order.
fn compact(part: &Part, indices: &[u32]) -> Part {
    let mut remap = vec![u32::MAX; part.v.len()];
    let mut out = Part::default();
    for &i in indices {
        let r = &mut remap[i as usize];
        if *r == u32::MAX {
            *r = out.v.len() as u32;
            out.v.push(part.v[i as usize]);
        }
        out.i.push(*r);
    }
    out
}

fn srgb_byte(linear: f32) -> u8 {
    (crate::tree::srgb(linear.clamp(0., 1.)) * 255.).round() as u8
}

fn linear(byte: u8) -> f32 {
    ((byte as f32 / 255. + 0.055) / 1.055).powf(2.4)
}

/// The atlas without its sprays: opaque leaf columns and alpha-cut leaf columns, both sampled
/// from the 256 x 256 leaf colours.
fn leaf_atlas(colors: &[u8], kind: LeafKind) -> Vec<u8> {
    let edge = ATLAS as usize;
    let mut out = vec![0u8; edge * edge * 4];
    let sample = |u: f32, v: f32| {
        let x = ((u * 256.) as usize).min(255);
        let y = ((v * 256.) as usize).min(255);
        let i = (y * 256 + x) * 4;
        [colors[i], colors[i + 1], colors[i + 2]]
    };
    for y in 0..edge {
        let vy = (y as f32 + 0.5) / edge as f32;
        for x in 0..edge {
            let u = (x as f32 + 0.5) / edge as f32;
            let i = (y * edge + x) * 4;
            for (region, cut) in [(OPAQUE, false), (CUTOUT, true)] {
                if vy < region.0 || vy > region.1 {
                    continue;
                }
                let v = (vy - region.0) / (region.1 - region.0);
                let rgb = sample(u, v);
                out[i..i + 3].copy_from_slice(&rgb);
                let alpha = if cut {
                    let across = (((u * 4.).fract()) - 0.5) / 0.47;
                    let limit = if kind == LeafKind::Needles {
                        1.0 - v * 0.9
                    } else {
                        2. * kind.half_width(v)
                    };
                    across.abs() <= limit
                } else {
                    true
                };
                out[i + 3] = if alpha { 255 } else { 0 };
            }
        }
    }
    out
}

/// Bakes two spray cards into the atlas, each the leaves of a real cluster of the tree seen
/// along the cluster's normal, darker where they lie deeper.
fn bake_sprays(
    atlas: &mut [u8],
    colors: &[u8],
    part: &Part,
    verts: usize,
    leaves: &[Leaf],
    clusters: &[Cluster],
) {
    // two well-filled clusters near the middle of the crown's spread
    let mut order: Vec<usize> = (0..clusters.len()).collect();
    order.sort_by_key(|&k| std::cmp::Reverse(clusters[k].members.len()));
    let picks = [order[order.len() / 6], order[(order.len() / 3).min(order.len() - 1)]];
    let edge = ATLAS as usize;
    let tris: &[[usize; 3]] = if verts == 3 {
        &[[0, 2, 1]]
    } else {
        &[[0, 2, 1], [0, 3, 2], [1, 2, 5], [1, 5, 4], [2, 3, 6], [2, 6, 5], [4, 5, 7], [5, 6, 7]]
    };
    for (slot, &pick) in picks.iter().enumerate() {
        let c = &clusters[pick];
        let (t, b) = orthonormal(c.normal);
        let (x0, x1) = (
            (SPRAY_U[slot].0 * edge as f32) as usize,
            (SPRAY_U[slot].1 * edge as f32) as usize,
        );
        let (y0, y1) = ((SPRAY.0 * edge as f32) as usize, (SPRAY.1 * edge as f32) as usize);
        let (w, h) = ((x1 - x0) as f32, (y1 - y0) as f32);
        let mut depth = vec![f32::MIN; (x1 - x0) * (y1 - y0)];
        // the cluster's own leaves, plus any neighbour reaching into the card
        let reach = c.radius;
        let near: Vec<usize> = (0..leaves.len())
            .filter(|&k| leaves[k].center().distance(c.center) < reach * 1.2)
            .collect();
        let deepest = near
            .iter()
            .map(|&k| (leaves[k].center() - c.center).dot(c.normal))
            .fold(0., |a: f32, d| a.max(d.abs()))
            .max(1e-3);
        for &k in &near {
            let piece = &part.v[k * verts..k * verts + verts];
            // needle fans are thin slivers: drawn twice their size they read as the dense
            // shoot a spray card stands for at its distance
            let fan = if verts == 3 { 2.0 } else { 1.0 };
            let middle = piece.iter().map(|v| v.p).sum::<Vec3>() / verts as f32;
            let project = |v: &V| {
                let d = middle + (v.p - middle) * fan - c.center;
                (
                    vec2(
                        (d.dot(t) / reach * 0.5 + 0.5) * w,
                        (d.dot(b) / reach * 0.5 + 0.5) * h,
                    ),
                    d.dot(c.normal),
                )
            };
            for tri in tris {
                let [a, bb, cc] = tri.map(|j| project(&piece[j]));
                let [ua, ub, uc] = tri.map(|j| piece[j].uv);
                raster(a, bb, cc, |px, py, l0, l1, l2| {
                    if px >= w as usize || py >= h as usize {
                        return;
                    }
                    let z = a.1 * l0 + bb.1 * l1 + cc.1 * l2;
                    let slotd = &mut depth[py * (x1 - x0) + px];
                    if z <= *slotd {
                        return;
                    }
                    *slotd = z;
                    let uv: Vec2 = ua * l0 + ub * l1 + uc * l2;
                    let sx = ((uv.x * 256.) as usize).min(255);
                    let sy = ((uv.y * 256.) as usize).min(255);
                    let si = (sy * 256 + sx) * 4;
                    // deeper leaves are in the shade of the ones above them
                    let shade = 0.6 + 0.4 * ((z / deepest) * 0.5 + 0.5).clamp(0., 1.);
                    let o = ((y0 + py) * edge + x0 + px) * 4;
                    for ch in 0..3 {
                        atlas[o + ch] = srgb_byte(linear(colors[si + ch]) * shade);
                    }
                    atlas[o + 3] = 255;
                });
            }
        }
    }
}

/// Fills a 2-D triangle, calling `f(x, y, l0, l1, l2)` with barycentric weights per pixel.
fn raster(
    a: (Vec2, f32),
    b: (Vec2, f32),
    c: (Vec2, f32),
    mut f: impl FnMut(usize, usize, f32, f32, f32),
) {
    let (pa, pb, pc) = (a.0, b.0, c.0);
    let area = (pb - pa).perp_dot(pc - pa);
    if area.abs() < 1e-9 {
        return;
    }
    let lo = pa.min(pb).min(pc).max(Vec2::ZERO);
    let hi = pa.max(pb).max(pc);
    for y in lo.y.floor() as i32..=hi.y.ceil() as i32 {
        for x in lo.x.floor() as i32..=hi.x.ceil() as i32 {
            if x < 0 || y < 0 {
                continue;
            }
            let p = vec2(x as f32 + 0.5, y as f32 + 0.5);
            let l0 = (pb - p).perp_dot(pc - p) / area;
            let l1 = (pc - p).perp_dot(pa - p) / area;
            let l2 = 1. - l0 - l1;
            if l0 >= 0. && l1 >= 0. && l2 >= 0. {
                f(x as usize, y as usize, l0, l1, l2);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trimming_preserves_opaque_pieces_and_clips_empty_card_corners() {
        let vertex = |u: f32, v: f32| V { p: Vec3::new(u, v, 0.), n: Vec3::Z, uv: vec2(u,v), c: [1.; 4] };
        let mut atlas = vec![0; (ATLAS * ATLAS * 4) as usize];
        for y in 170..190 { for x in 45..75 { atlas[((y * ATLAS + x) * 4 + 3) as usize] = 255; } }
        let leaves = Part { v: vec![vertex(0.01, CUTOUT.0), vertex(0.24, CUTOUT.0), vertex(0.24, CUTOUT.1), vertex(0.01, CUTOUT.1),
                                      vertex(0.01, OPAQUE.0), vertex(0.1, OPAQUE.0), vertex(0.1, OPAQUE.1)],
                            i: vec![0,1,2, 0,2,3, 4,5,6] };
        let mut ladder = Ladder { atlas, levels: vec![Level { rung: LADDER[0], bark: Part::default(), leaves: Part::default() }, Level { rung: LADDER[1], bark: Part::default(), leaves }] };
        trim_cards(&mut ladder);
        let out = &ladder.levels[1].leaves;
        assert!(out.i.len() >= 9);
        assert!(out.v.iter().any(|v| v.uv.y == OPAQUE.0));
        for v in out.v.iter().filter(|v| v.uv.y >= CUTOUT.0) {
            assert!(v.uv.x >= 40. / ATLAS as f32 && v.uv.x <= 80. / ATLAS as f32);
            assert!(v.p.distance(Vec3::new(v.uv.x, v.uv.y, 0.)) < 1e-6);
        }
    }

    #[test]
    fn every_rung_meets_its_budget_and_keeps_the_ground_origin() {
        for id in ["solstice-oak", "solstice-fir", "solstice-cwa-buk", "solstice-cwa-smrk"] {
            let s = Settings::preset(id).unwrap();
            let l = ladder(&s);
            assert_eq!(l.levels.len(), LADDER.len());
            let mut last = usize::MAX;
            for level in &l.levels {
                let tris = level.triangles();
                assert!(tris <= level.rung.budget + level.rung.budget / 10, "{id} {}: {tris}", level.rung.name);
                assert!(tris < last, "{id}: levels must shrink");
                last = tris;
                assert!(!level.bark.i.is_empty() && !level.leaves.i.is_empty(), "{id} {}", level.rung.name);
                // the trunk's foot stays on the origin at every level
                let foot = trunk_foot(&level.bark);
                assert!(foot.abs() < 0.05, "{id} {}: trunk foot at {foot}", level.rung.name);
                for p in [&level.bark, &level.leaves] {
                    assert!(p.i.iter().all(|&i| (i as usize) < p.v.len()));
                    assert!(p.v.iter().all(|v| v.p.is_finite() && v.n.is_finite()));
                }
            }
            assert_eq!(l.atlas.len(), (ATLAS * ATLAS * 4) as usize);
        }
    }
}
