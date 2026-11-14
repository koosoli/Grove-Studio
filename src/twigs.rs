//! Baked twig cards (`--twig-cards`): a game LOD ladder that stays full at Reforger-style
//! triangle budgets (8k / 3k / 900 / 250 triangles).
//!
//! At such budgets the classic ladder thins single leaves until the crown reads as a sapling.
//! Here the thin wood and its leaves are not drawn as geometry at all: every leafy twig (the
//! subtree under a branch thinner than a radius cutoff) becomes one or two alpha cards whose
//! texture is a real twig of the same tree, rendered orthographically into an atlas with its
//! colour, coverage and tangent-space normal. Thousands of leaves then cost a few triangles.
//!
//! | level    | wood                         | foliage                                         |
//! |----------|------------------------------|-------------------------------------------------|
//! | near     | branches above the cutoff    | twig cards (front + crossed side view)          |
//! | mid      | thicker cutoff               | branch cards baked from larger subtrees         |
//! | far      | trunk and limbs              | limb cards                                      |
//! | very far | none (baked into the cards)  | two crossed whole-tree cards, ellipsoid normals |
//!
//! The cutoff of each level is searched so its cards fill the level's budget, and what the
//! cards leave is given to the wood, so a level lands close to its budget instead of at ~60 %.
//! Atlas: `EDGE` x `EDGE`, a 4 x 4 grid of `CELL` cells (near 4 variants x 2 views, mid 2 x 2,
//! far 1 x 2, very far 1 x 2).
use crate::gamelod::{self, Ladder, Level, Rung, LADDER};
use crate::tree::{Part, Settings, V};
use macroquad::prelude::{vec2, Vec2, Vec3};
use std::collections::HashMap;

pub const EDGE: u32 = 2048;
pub const CELL: u32 = 512;
/// Supersampling per axis while baking.
const SS: u32 = 3;
/// The default budgets of `--twig-cards`, in triangles as Reforger counts them (one per card
/// triangle). The P3D also writes a reversed face per foliage triangle, so it holds more faces.
pub const BUDGETS: [usize; 4] = [8_000, 3_000, 900, 250];

/// The baked atlas: `color` is sRGB RGBA with coverage in alpha; `normal` is the tangent-space
/// normal in OpenPoseidon's `_nohq` layout (R 0, G y, B z, A x), X along +U and Y along -V
/// (up in the image), as the bark normal maps.
pub struct TwigAtlas {
    pub edge: u32,
    pub color: Vec<u8>,
    pub normal: Vec<u8>,
    /// Debug: per level, how many groups (cards' subtrees) and the radius cutoff used.
    pub report: Vec<serde_json::Value>,
}

#[derive(Clone, Copy)]
struct Cell {
    x: u32,
    y: u32,
}

impl Cell {
    fn slot(k: u32) -> Cell {
        Cell { x: (k % (EDGE / CELL)) * CELL, y: (k / (EDGE / CELL)) * CELL }
    }
    fn uv(&self, s: f32, t: f32) -> Vec2 {
        vec2((self.x as f32 + s * CELL as f32) / EDGE as f32, (self.y as f32 + t * CELL as f32) / EDGE as f32)
    }
}

/// An orthographic view of a subtree: image X along `t`, image up along `b`, the viewer on the
/// `n` side (`t x b = n`); extents relative to `o`.
#[derive(Clone, Copy)]
struct View {
    o: Vec3,
    t: Vec3,
    b: Vec3,
    n: Vec3,
    x: (f32, f32),
    y: (f32, f32),
    z: (f32, f32),
}

impl View {
    fn local(&self, p: Vec3) -> Vec3 {
        let d = p - self.o;
        Vec3::new(d.dot(self.t), d.dot(self.b), d.dot(self.n))
    }
    fn fit(o: Vec3, t: Vec3, b: Vec3, points: &[Vec3]) -> View {
        let n = t.cross(b);
        let mut v = View { o, t, b, n, x: (f32::MAX, f32::MIN), y: (f32::MAX, f32::MIN), z: (f32::MAX, f32::MIN) };
        for &p in points {
            let l = v.local(p);
            v.x = (v.x.0.min(l.x), v.x.1.max(l.x));
            v.y = (v.y.0.min(l.y), v.y.1.max(l.y));
            v.z = (v.z.0.min(l.z), v.z.1.max(l.z));
        }
        if points.is_empty() { v.x = (-0.01, 0.01); v.y = v.x; v.z = v.x; }
        for r in [&mut v.x, &mut v.y] {
            if r.1 - r.0 < 0.02 { let m = (r.0 + r.1) * 0.5; *r = (m - 0.01, m + 0.01); }
        }
        v
    }
    /// The same view seen from the other side: mirrored image, opposite normal.
    fn mirrored(&self) -> View {
        View { t: -self.t, n: -self.n, x: (-self.x.1, -self.x.0), z: (-self.z.1, -self.z.0), ..*self }
    }
    /// The world position of image point (s, t) in 0..1 (t down) on the card plane.
    fn at(&self, s: f32, t: f32) -> Vec3 {
        let x = self.x.0 + s * (self.x.1 - self.x.0);
        let y = self.y.1 - t * (self.y.1 - self.y.0);
        self.o + self.t * x + self.b * y + self.n * ((self.z.0 + self.z.1) * 0.5)
    }
}

/// The subtree a card stands for: leaf and bark triangles (indices of whole triangles).
struct Group {
    owner: u32,
    root: Vec3,
    leaf_tris: Vec<u32>,
    bark_tris: Vec<u32>,
    /// A real twig (a branch and its children), not a chunk of leaves on a thick branch.
    twig: bool,
}

struct Tree<'a> {
    leaves: &'a Part,
    bark: &'a Part,
    branches: &'a [crate::tree::Branch],
    children: Vec<Vec<usize>>,
    radius: Vec<f32>,
    leaf_by_branch: Vec<Vec<u32>>,
    bark_by_branch: Vec<Vec<u32>>,
    colors: Vec<u8>,
    bark_rgba: (u32, u32, Vec<u8>),
    height: f32,
    crown: Vec3,
    crown_radii: Vec3,
    /// Needle fans are slivers: baked at twice their size they read as the dense shoot.
    needles: bool,
}

impl Tree<'_> {
    fn group_points(&self, g: &Group) -> Vec<Vec3> {
        let mut points = vec![g.root];
        for &t in &g.leaf_tris {
            points.extend((0..3).map(|k| self.leaves.v[self.leaves.i[t as usize * 3 + k] as usize].p));
        }
        for &t in &g.bark_tris {
            points.extend((0..3).map(|k| self.bark.v[self.bark.i[t as usize * 3 + k] as usize].p));
        }
        points
    }

    /// The groups at a branch radius cutoff, and whether each branch stays as wood.
    fn groups_at(&self, cutoff: f32) -> (Vec<Group>, Vec<bool>) {
        let nb = self.branches.len();
        let mut member = vec![usize::MAX; nb];
        let mut groups = Vec::new();
        for b in 0..nb {
            let thin = self.radius[b] < cutoff;
            let parent_thin = self.branches[b].parent.is_some_and(|p| self.radius[p] < cutoff);
            if !thin || parent_thin || member[b] < usize::MAX - 1 { continue; }
            let k = groups.len();
            let mut stack = vec![b];
            let mut g = Group { owner: b as u32, root: self.branches[b].points.first().map_or(Vec3::ZERO, |p| Vec3::from_array(*p)),
                leaf_tris: Vec::new(), bark_tris: Vec::new(), twig: true };
            let mut members = Vec::new();
            while let Some(c) = stack.pop() {
                if member[c] != usize::MAX { continue; }
                member[c] = k;
                members.push(c);
                g.leaf_tris.extend_from_slice(&self.leaf_by_branch[c]);
                g.bark_tris.extend_from_slice(&self.bark_by_branch[c]);
                stack.extend_from_slice(&self.children[c]);
            }
            if g.leaf_tris.is_empty() {
                // a bare twig has nothing a leafy card could stand for: it stays wood
                for c in members { member[c] = usize::MAX - 1; }
                continue;
            }
            groups.push(g);
        }
        let wood: Vec<bool> = member.iter().map(|&m| m >= usize::MAX - 1).collect();
        // Leaves growing straight from wood that stays: chunks of about a twig's size.
        let mut diag: Vec<f32> = groups.iter().filter(|g| !g.leaf_tris.is_empty()).map(|g| {
            let p = self.group_points(g);
            let lo = p.iter().fold(Vec3::splat(f32::MAX), |a, &b| a.min(b));
            let hi = p.iter().fold(Vec3::splat(f32::MIN), |a, &b| a.max(b));
            (hi - lo).length()
        }).collect();
        diag.sort_by(f32::total_cmp);
        let size = diag.get(diag.len() / 2).copied().unwrap_or(self.height * 0.12).max(0.05) * 0.8;
        let mut chunks: HashMap<(i32, i32, i32), Vec<u32>> = HashMap::new();
        for b in (0..nb).filter(|&b| wood[b]) {
            for &t in &self.leaf_by_branch[b] {
                let c = (0..3).map(|k| self.leaves.v[self.leaves.i[t as usize * 3 + k] as usize].p).sum::<Vec3>() / 3. / size;
                chunks.entry((c.x.floor() as i32, c.y.floor() as i32, c.z.floor() as i32)).or_default().push(t);
            }
        }
        let mut keys: Vec<_> = chunks.keys().copied().collect();
        keys.sort();
        for key in keys {
            let tris = chunks.remove(&key).unwrap();
            let mut count: HashMap<u32, usize> = HashMap::new();
            for &t in &tris { *count.entry(self.leaves.v[self.leaves.i[t as usize * 3] as usize].branch).or_default() += 1; }
            let owner = count.iter().max_by_key(|(b, n)| (**n, std::cmp::Reverse(**b))).map(|(b, _)| *b).unwrap();
            let centre = tris.iter().map(|&t| self.leaves.v[self.leaves.i[t as usize * 3] as usize].p).sum::<Vec3>() / tris.len() as f32;
            let root = self.branches[owner as usize].points.iter().map(|p| Vec3::from_array(*p))
                .min_by(|a, b| a.distance(centre).total_cmp(&b.distance(centre))).unwrap_or(centre);
            groups.push(Group { owner, root, leaf_tris: tris, bark_tris: Vec::new(), twig: false });
        }
        (groups, wood)
    }

    /// Merges neighbouring groups on a grid until at most `target` remain.
    fn merge(&self, groups: Vec<Group>, target: usize) -> Vec<Group> {
        if groups.len() <= target { return groups; }
        let centres: Vec<Vec3> = groups.iter().map(|g| {
            let p = self.group_points(g);
            p.iter().sum::<Vec3>() / p.len() as f32
        }).collect();
        let key = |c: Vec3, cell: f32| ((c.x / cell).floor() as i32, (c.y / cell).floor() as i32, (c.z / cell).floor() as i32);
        let count = |cell: f32| centres.iter().map(|&c| key(c, cell)).collect::<std::collections::HashSet<_>>().len();
        let (mut lo, mut hi) = (0.01f32, self.height * 4.);
        for _ in 0..30 {
            let mid = (lo * hi).sqrt();
            if count(mid) > target { lo = mid } else { hi = mid }
        }
        let mut cells: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
        for (g, &c) in centres.iter().enumerate() { cells.entry(key(c, hi)).or_default().push(g); }
        let mut keys: Vec<_> = cells.keys().copied().collect();
        keys.sort();
        let mut slots: Vec<Option<Group>> = groups.into_iter().map(Some).collect();
        keys.into_iter().map(|k| {
            let mut members: Vec<Group> = cells[&k].iter().map(|&g| slots[g].take().unwrap()).collect();
            members.sort_by_key(|g| std::cmp::Reverse(g.leaf_tris.len()));
            let mut first = members.remove(0);
            for m in members {
                first.leaf_tris.extend(m.leaf_tris);
                first.bark_tris.extend(m.bark_tris);
                first.twig |= m.twig;
            }
            first
        }).collect()
    }

    /// The front view (along the subtree's thinnest axis) and the crossed side view, both
    /// turned to face out of the crown.
    fn views(&self, g: &Group) -> [View; 2] {
        let points = self.group_points(g);
        let centre = points.iter().sum::<Vec3>() / points.len() as f32;
        let (_, axes) = principal_axes(&points, centre);
        let mut u = axes[0];
        let away = centre - g.root;
        if away.length_squared() > 1e-8 {
            if u.dot(away) < 0. { u = -u; }
        } else if u.y < 0. { u = -u; }
        let w = axes[2];
        let v = w.cross(u).normalize_or_zero();
        let front = View::fit(centre, u, v, &points);
        let mut side = View::fit(centre, u, w, &points);
        // a flat spray seen edge-on is a sliver: give the crossed card some body
        let min_width = 0.4 * (front.y.1 - front.y.0);
        if side.y.1 - side.y.0 < min_width {
            let m = (side.y.0 + side.y.1) * 0.5;
            side.y = (m - min_width * 0.5, m + min_width * 0.5);
        }
        let out = (centre - self.crown) / self.crown_radii;
        [front, side].map(|view| if view.n.dot(out) < 0. { view.mirrored() } else { view })
    }

    /// Normal of the crown's ellipsoid through `p`.
    fn crown_normal(&self, p: Vec3) -> Vec3 {
        let r2 = self.crown_radii * self.crown_radii;
        ((p - self.crown) / r2).normalize_or(Vec3::Y)
    }
}

/// Eigenvectors of the point covariance, largest spread first.
fn principal_axes(points: &[Vec3], centre: Vec3) -> ([f32; 3], [Vec3; 3]) {
    let mut m = [[0f64; 3]; 3];
    for p in points {
        let d = (*p - centre).to_array();
        for i in 0..3 { for j in 0..3 { m[i][j] += d[i] as f64 * d[j] as f64; } }
    }
    let mut v = [[1f64, 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for _ in 0..32 {
        let (mut p, mut q, mut best) = (0, 1, 0f64);
        for i in 0..3 { for j in i + 1..3 { if m[i][j].abs() > best { best = m[i][j].abs(); p = i; q = j; } } }
        if best < 1e-14 { break; }
        let theta = (m[q][q] - m[p][p]) / (2. * m[p][q]);
        let t = theta.signum() / (theta.abs() + (theta * theta + 1.).sqrt());
        let t = if theta == 0. { 1. } else { t };
        let c = 1. / (t * t + 1.).sqrt();
        let s = t * c;
        for k in 0..3 {
            let (a, b) = (m[k][p], m[k][q]);
            m[k][p] = c * a - s * b;
            m[k][q] = s * a + c * b;
        }
        for k in 0..3 {
            let (a, b) = (m[p][k], m[q][k]);
            m[p][k] = c * a - s * b;
            m[q][k] = s * a + c * b;
        }
        for row in &mut v {
            let (a, b) = (row[p], row[q]);
            row[p] = c * a - s * b;
            row[q] = s * a + c * b;
        }
    }
    let mut order = [0usize, 1, 2];
    order.sort_by(|&a, &b| m[b][b].total_cmp(&m[a][a]));
    let vec = |k: usize| Vec3::new(v[0][k] as f32, v[1][k] as f32, v[2][k] as f32).normalize_or(Vec3::Y);
    let mut axes = order.map(vec);
    // right-handed and orthonormal
    axes[2] = axes[0].cross(axes[1]).normalize_or(Vec3::Z);
    axes[1] = axes[2].cross(axes[0]).normalize_or(Vec3::Y);
    (order.map(|k| m[k][k] as f32), axes)
}

/// Renders the triangles of a subtree in `view` into `cell` of the atlas.
fn bake(tree: &Tree, view: &View, leaf_tris: &[u32], bark_tris: &[u32], atlas: &mut TwigAtlas, cell: Cell) {
    let size = (CELL * SS) as usize;
    let mut depth = vec![f32::MIN; size * size];
    let mut color = vec![[0f32; 3]; size * size];
    let mut normal = vec![Vec3::ZERO; size * size];
    let zr = (view.z.0, (view.z.1 - view.z.0).max(1e-4));
    let project = |p: Vec3| {
        let l = view.local(p);
        (vec2((l.x - view.x.0) / (view.x.1 - view.x.0) * size as f32, (view.y.1 - l.y) / (view.y.1 - view.y.0) * size as f32), l.z)
    };
    for (part, tris, leaf) in [(tree.bark, bark_tris, false), (tree.leaves, leaf_tris, true)] {
        for &t in tris {
            let mut vs: [V; 3] = std::array::from_fn(|k| part.v[part.i[t as usize * 3 + k] as usize]);
            if leaf && tree.needles {
                let middle = (vs[0].p + vs[1].p + vs[2].p) / 3.;
                for v in &mut vs { v.p = middle + (v.p - middle) * 2.; }
            }
            let [a, b, c] = vs.map(|v| project(v.p));
            gamelod::raster(a, b, c, |px, py, l0, l1, l2| {
                if px >= size || py >= size { return; }
                let i = py * size + px;
                let z = a.1 * l0 + b.1 * l1 + c.1 * l2;
                if z <= depth[i] { return; }
                let uv = vs[0].uv * l0 + vs[1].uv * l1 + vs[2].uv * l2;
                let albedo = if leaf {
                    let sx = ((uv.x * 256.) as usize).min(255);
                    let sy = ((uv.y * 256.) as usize).min(255);
                    let si = (sy * 256 + sx) * 4;
                    if tree.colors[si + 3] < 128 { return; }
                    [0, 1, 2].map(|ch| gamelod::linear(tree.colors[si + ch]))
                } else {
                    let (w, h, px) = &tree.bark_rgba;
                    let sx = ((uv.x.rem_euclid(1.) * *w as f32) as u32).min(w - 1);
                    let sy = ((uv.y.rem_euclid(1.) * *h as f32) as u32).min(h - 1);
                    let si = ((sy * w + sx) * 4) as usize;
                    [0, 1, 2].map(|ch| gamelod::linear(px[si + ch]))
                };
                depth[i] = z;
                // the twig's own depth: its far side is shaded by its near side
                let shade = 0.72 + 0.28 * ((z - zr.0) / zr.1).clamp(0., 1.);
                color[i] = albedo.map(|x| x * shade);
                let mut n = (vs[0].n * l0 + vs[1].n * l1 + vs[2].n * l2).normalize_or(view.n);
                if n.dot(view.n) < 0. { n = -n; }
                normal[i] = Vec3::new(n.dot(view.t), n.dot(view.b), n.dot(view.n));
            });
        }
    }
    let edge = atlas.edge as usize;
    let mut texels = vec![None::<([f32; 3], Vec3)>; (CELL * CELL) as usize];
    for ty in 0..CELL as usize {
        for tx in 0..CELL as usize {
            let (mut sum, mut nsum, mut k) = ([0f32; 3], Vec3::ZERO, 0);
            for sy in 0..SS as usize {
                for sx in 0..SS as usize {
                    let i = (ty * SS as usize + sy) * size + tx * SS as usize + sx;
                    if depth[i] > f32::MIN {
                        k += 1;
                        for ch in 0..3 { sum[ch] += color[i][ch]; }
                        nsum += normal[i];
                    }
                }
            }
            let o = ((cell.y as usize + ty) * edge + cell.x as usize + tx) * 4;
            let coverage = k as f32 / (SS * SS) as f32;
            atlas.color[o + 3] = (coverage * 255.).round() as u8;
            if k > 0 {
                texels[ty * CELL as usize + tx] = Some((sum.map(|x| x / k as f32), nsum.normalize_or(Vec3::Z)));
            }
        }
    }
    // Bleed colour and normal into the empty texels, so mipmaps and BC blocks do not pull in
    // black at the outline.
    for _ in 0..12 {
        let previous = texels.clone();
        let cw = CELL as usize;
        for y in 0..cw {
            for x in 0..cw {
                if previous[y * cw + x].is_some() { continue; }
                let (mut sum, mut nsum, mut k) = ([0f32; 3], Vec3::ZERO, 0);
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx < 0 || ny < 0 || nx >= cw as i32 || ny >= cw as i32 { continue; }
                    if let Some((c, n)) = previous[ny as usize * cw + nx as usize] {
                        k += 1;
                        for ch in 0..3 { sum[ch] += c[ch]; }
                        nsum += n;
                    }
                }
                if k > 0 { texels[y * cw + x] = Some((sum.map(|s| s / k as f32), nsum.normalize_or(Vec3::Z))); }
            }
        }
    }
    let fallback = texels.iter().flatten().next().map_or([0.05, 0.1, 0.03], |t| t.0);
    for ty in 0..CELL as usize {
        for tx in 0..CELL as usize {
            let o = ((cell.y as usize + ty) * edge + cell.x as usize + tx) * 4;
            let (c, n) = texels[ty * CELL as usize + tx].unwrap_or((fallback, Vec3::Z));
            for ch in 0..3 { atlas.color[o + ch] = gamelod::srgb_byte(c[ch]); }
            let n = Vec3::new(n.x, n.y, n.z.max(0.05)).normalize();
            let byte = |c: f32| ((c * 0.5 + 0.5) * 255.).round().clamp(0., 255.) as u8;
            atlas.normal[o..o + 4].copy_from_slice(&[0, byte(n.y), byte(n.z), byte(n.x)]);
        }
    }
}

/// The card outline of a baked cell in cell coordinates (0..1, t down): the trimmed hull, or
/// the whole cell.
fn outline(atlas: &TwigAtlas, cell: Cell, trim: bool) -> Vec<Vec2> {
    let full = vec![vec2(0., 1.), vec2(1., 1.), vec2(1., 0.), vec2(0., 0.)];
    if !trim { return full; }
    let edge = atlas.edge;
    let hull = gamelod::alpha_outline(&|x, y| atlas.color[((y * edge + x) * 4 + 3) as usize] >= 96,
        (cell.x, cell.x + CELL), (cell.y, cell.y + CELL), edge, 2.);
    if hull.len() < 3 { return full; }
    // to cell coordinates, clipped to the cell
    let mut poly: Vec<Vec2> = hull.iter().map(|p| vec2((p.x * edge as f32 - cell.x as f32) / CELL as f32,
        (p.y * edge as f32 - cell.y as f32) / CELL as f32)).collect();
    for (axis, limit, keep_below) in [(0, 0., false), (0, 1., true), (1, 0., false), (1, 1., true)] {
        let input = std::mem::take(&mut poly);
        if input.is_empty() { break; }
        let inside = |p: Vec2| { let c = if axis == 0 { p.x } else { p.y }; if keep_below { limit - c } else { c - limit } };
        let mut previous = *input.last().unwrap();
        for current in input {
            let (dp, dc) = (inside(previous), inside(current));
            if (dp >= 0.) != (dc >= 0.) { poly.push(previous.lerp(current, dp / (dp - dc))); }
            if dc >= 0. { poly.push(current); }
            previous = current;
        }
    }
    poly.dedup_by(|a, b| a.distance_squared(*b) < 1e-10);
    if poly.len() >= 3 { poly } else { full }
}

/// Appends one card: `shape` (cell coordinates) on `view`'s plane, UVs into `cell`.
fn emit(out: &mut Part, tree: &Tree, view: &View, cell: Cell, shape: &[Vec2], owner: u32, bend: f32) {
    let first = out.v.len() as u32;
    for &s in shape {
        let p = view.at(s.x, s.y);
        let n = (view.n * (1. - bend) + tree.crown_normal(p) * bend).normalize_or(view.n);
        out.v.push(V { p, n, c: [1.; 4], uv: cell.uv(s.x, s.y), branch: owner });
    }
    for k in 1..shape.len() as u32 - 1 {
        let (a, b, c) = (first, first + k, first + k + 1);
        let pa = out.v[a as usize].p;
        let face = (out.v[b as usize].p - pa).cross(out.v[c as usize].p - pa);
        // counter-clockwise seen from the card's normal, as the other foliage
        out.i.extend(if face.dot(view.n) >= 0. { [a, b, c] } else { [a, c, b] });
    }
}

struct LevelSpec {
    variants: usize,
    views: usize,
    first_slot: u32,
}

/// The `--twig-cards` ladder. `budgets` are triangles per level (default `BUDGETS`).
pub fn ladder(s: &Settings, budgets: Option<&[usize]>, trim: bool) -> Result<Ladder, String> {
    let budgets: Vec<usize> = budgets.map_or(BUDGETS.to_vec(), |b| b.to_vec());
    if budgets.len() != 4 || budgets.windows(2).any(|w| w[1] >= w[0]) || budgets[3] < 24 {
        return Err("--twig-cards --budgets wants four shrinking triangle counts, the last at least 24".into());
    }
    let mut full = crate::tree::generate(s, 0);
    let foot = gamelod::trunk_foot(&full.bark);
    for v in full.bark.v.iter_mut().chain(&mut full.leaves.v) { v.p.y -= foot; }
    for branch in &mut full.branches { for point in &mut branch.points { point[1] -= foot; } }
    let nb = full.branches.len();
    let mut children = vec![Vec::new(); nb];
    for (b, branch) in full.branches.iter().enumerate() {
        if let Some(p) = branch.parent { children[p].push(b); }
    }
    let by_branch = |part: &Part| {
        let mut out = vec![Vec::new(); nb];
        for t in 0..part.i.len() / 3 {
            let b = part.v[part.i[t * 3] as usize].branch as usize;
            if b < nb { out[b].push(t as u32); }
        }
        out
    };
    let bark_png = crate::tree::bark_texture(s)?;
    let bark_image = image::load_from_memory(&bark_png).map_err(|e| e.to_string())?.into_rgba8();
    let (lo, hi) = full.leaves.v.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(a, b), v| (a.min(v.p), b.max(v.p)));
    let height = full.bounds().1.y.max(1.0);
    let (crown, crown_radii) = if full.leaves.v.is_empty() {
        (Vec3::Y * height * 0.6, Vec3::splat(height * 0.3))
    } else {
        ((lo + hi) * 0.5, ((hi - lo) * 0.5).max(Vec3::splat(0.3)))
    };
    let tree = Tree {
        leaves: &full.leaves,
        bark: &full.bark,
        branches: &full.branches,
        radius: full.branches.iter().map(|b| b.radii.first().copied().unwrap_or(0.)).collect(),
        leaf_by_branch: by_branch(&full.leaves),
        bark_by_branch: by_branch(&full.bark),
        children,
        colors: crate::preview::leaf_rgba(s),
        bark_rgba: (bark_image.width(), bark_image.height(), bark_image.into_raw()),
        height,
        crown,
        crown_radii,
        needles: crate::reference::leaf_kind(s) == Some(solstice_procgen::foliage::leaf::LeafKind::Needles),
    };
    let edge = EDGE as usize;
    let mut atlas = TwigAtlas { edge: EDGE, color: vec![0; edge * edge * 4], normal: vec![0; edge * edge * 4], report: Vec::new() };
    for p in atlas.normal.chunks_exact_mut(4) { p.copy_from_slice(&[0, 128, 255, 128]); }
    let specs = [
        LevelSpec { variants: 4, views: 2, first_slot: 0 },
        LevelSpec { variants: 2, views: 2, first_slot: 8 },
        LevelSpec { variants: 1, views: 2, first_slot: 12 },
    ];
    // every radius a cutoff can sit at, thinnest first
    let mut radii: Vec<f32> = tree.radius.clone();
    radii.push(f32::MAX);
    radii.sort_by(f32::total_cmp);
    radii.dedup();
    let mut levels = Vec::new();
    let mut previous_cutoff = 0f32;
    for (k, spec) in specs.iter().enumerate() {
        let mut rung: Rung = LADDER[k];
        rung.budget = budgets[k];
        rung.min_branch = 0.;
        // an untrimmed card is 2 triangles, a trimmed one at most 6 (usually 4)
        let faces_per_group = spec.views * if trim { 5 } else { 2 };
        let wood_share = (rung.budget as f64 * LADDER[k].wood as f64 / LADDER[k].budget as f64) as usize;
        let total_leaf_tris = full.leaves.i.len() / 3;
        let fits = |cutoff: f32| {
            let (groups, wood) = tree.groups_at(cutoff);
            let raw: usize = (0..nb).filter(|&b| wood[b]).map(|b| tree.bark_by_branch[b].len()).sum();
            // below the thinnest twigs every leaf is a loose chunk again: not a twig card
            let loose: usize = groups.iter().filter(|g| !g.twig).map(|g| g.leaf_tris.len()).sum();
            groups.len() * faces_per_group + raw.min(wood_share) <= rung.budget && loose * 2 <= total_leaf_tris
        };
        // the thinnest cutoff (most, smallest cards) that still fits the budget
        let (mut a, mut b) = (0usize, radii.len() - 1);
        while a < b {
            let m = (a + b) / 2;
            if fits(radii[m]) { b = m } else { a = m + 1 }
        }
        // radii[a] is the first that fits; cut just above it so that radius is included
        let mut cutoff = if radii[a] == f32::MAX { f32::MAX } else { radii[a] * 1.0001 };
        let (mut groups, mut wood) = tree.groups_at(cutoff);
        if k > 0 && groups.len() < 8 {
            // No branch thickness splits the crown into few enough cards (a conifer's limbs all
            // grow from the trunk): keep the previous level's subtrees and merge neighbours.
            cutoff = previous_cutoff;
            (groups, wood) = tree.groups_at(cutoff);
        }
        let raw: usize = (0..nb).filter(|&b| wood[b]).map(|b| tree.bark_by_branch[b].len()).sum();
        let target = rung.budget.saturating_sub(raw.min(wood_share)) / faces_per_group;
        let groups = tree.merge(groups, target.max(1));
        previous_cutoff = cutoff;
        // the representatives: real twigs at quantiles of size, so leaves keep their scale
        let sizes: Vec<(usize, f32, f32)> = groups.iter().enumerate().map(|(g, group)| {
            let [front, _] = tree.views(group);
            let (w, h) = (front.x.1 - front.x.0, front.y.1 - front.y.0);
            (g, (w * h).sqrt(), w / h)
        }).collect();
        let mut candidates: Vec<&(usize, f32, f32)> = sizes.iter().filter(|(g, _, _)| groups[*g].twig && groups[*g].leaf_tris.len() >= 4).collect();
        if candidates.is_empty() { candidates = sizes.iter().filter(|(g, _, _)| !groups[*g].leaf_tris.is_empty()).collect(); }
        candidates.sort_by(|a, b| a.1.total_cmp(&b.1));
        let quantiles: &[f32] = match spec.variants { 1 => &[0.75], 2 => &[0.55, 0.85], _ => &[0.4, 0.6, 0.75, 0.9] };
        let mut reps: Vec<usize> = quantiles.iter().filter_map(|q| candidates.get(((candidates.len() as f32 - 1.) * q).round() as usize).map(|c| c.0)).collect();
        reps.dedup();
        let mut shapes = Vec::new();
        for (r, &g) in reps.iter().enumerate() {
            let views = tree.views(&groups[g]);
            for (v, view) in views.iter().enumerate().take(spec.views) {
                let cell = Cell::slot(spec.first_slot + (r * spec.views + v) as u32);
                bake(&tree, view, &groups[g].leaf_tris, &groups[g].bark_tris, &mut atlas, cell);
                shapes.push(outline(&atlas, cell, trim));
            }
        }
        let mut leaves = Part::default();
        if !reps.is_empty() {
            for (g, group) in groups.iter().enumerate() {
                let (_, size, aspect) = sizes[g];
                let r = (0..reps.len()).min_by(|&a, &b| {
                    let score = |r: usize| { let (_, s, asp) = sizes[reps[r]]; (size / s).ln().abs() + (aspect / asp).ln().abs() };
                    score(a).total_cmp(&score(b))
                }).unwrap();
                for (v, view) in tree.views(group).iter().enumerate().take(spec.views) {
                    let slot = r * spec.views + v;
                    emit(&mut leaves, &tree, view, Cell::slot(spec.first_slot + slot as u32), &shapes[slot], group.owner, 0.6);
                }
            }
        }
        // the wood that stays gets whatever the cards leave
        let kept: Vec<u32> = (0..nb).filter(|&b| wood[b]).flat_map(|b| tree.bark_by_branch[b].iter().flat_map(|&t| (0..3).map(move |j| t * 3 + j))).map(|i| full.bark.i[i as usize]).collect();
        let kept = Part { v: full.bark.v.clone(), i: kept };
        let mut bark = Part::default();
        let mut left = rung.budget.saturating_sub(leaves.i.len() / 3);
        for _ in 0..3 {
            rung.wood = left.max(12);
            bark = gamelod::simplify_wood(&kept, &rung, height);
            let total = bark.i.len() / 3 + leaves.i.len() / 3;
            if total <= rung.budget { break; }
            left = left.saturating_sub(total - rung.budget);
        }
        atlas.report.push(serde_json::json!({"level": rung.name, "cutoffRadius": cutoff, "groups": groups.len(),
            "twigGroups": groups.iter().filter(|g| g.twig).count(), "variants": reps.len(),
            "cardTriangles": leaves.i.len() / 3, "woodTriangles": bark.i.len() / 3}));
        levels.push(Level { rung, bark, leaves });
    }
    // very far: two crossed cards of the whole tree through the trunk, ellipsoid normals
    let mut rung: Rung = LADDER[3];
    rung.budget = budgets[3];
    let all_leaves: Vec<u32> = (0..full.leaves.i.len() as u32 / 3).collect();
    let all_bark: Vec<u32> = (0..full.bark.i.len() as u32 / 3).collect();
    let points: Vec<Vec3> = full.bark.v.iter().chain(&full.leaves.v).map(|v| v.p).collect();
    let mut leaves = Part::default();
    // a grid per card so the bent normals interpolate like a dome: 2 cards x 2 m^2 triangles
    let m = (1..=6).rev().find(|m| 4 * m * m <= rung.budget).unwrap_or(1);
    for (v, (t, b)) in [(Vec3::X, Vec3::Y), (-Vec3::Z, Vec3::Y)].into_iter().enumerate() {
        let mut view = View::fit(Vec3::ZERO, t, b, &points);
        view.z = (0., 0.);
        let cell = Cell::slot(14 + v as u32);
        bake(&tree, &View::fit(Vec3::ZERO, t, b, &points), &all_leaves, &all_bark, &mut atlas, cell);
        view.z = (0., 0.);
        let first = leaves.v.len() as u32;
        for j in 0..=m {
            for i in 0..=m {
                let (s, tt) = (i as f32 / m as f32, j as f32 / m as f32);
                let p = view.at(s, tt);
                leaves.v.push(V { p, n: tree.crown_normal(p), c: [1.; 4], uv: cell.uv(s, tt), branch: 0 });
            }
        }
        let w = m as u32 + 1;
        for j in 0..m as u32 {
            for i in 0..m as u32 {
                let (a, b2, c, d) = (first + j * w + i, first + j * w + i + 1, first + (j + 1) * w + i + 1, first + (j + 1) * w + i);
                // t down the image: (s right, t down) is clockwise seen from n, so reverse
                leaves.i.extend([a, c, b2, a, d, c]);
            }
        }
    }
    atlas.report.push(serde_json::json!({"level": rung.name, "cards": 2, "grid": m, "cardTriangles": leaves.i.len() / 3}));
    levels.push(Level { rung, bark: Part::default(), leaves });
    Ok(Ladder { levels, atlas: Vec::new(), branches: full.branches.clone(), twig: Some(atlas) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twig_card_ladders_fill_but_never_exceed_their_budgets() {
        for preset in ["solstice-oak", "solstice-fir"] {
            let s = Settings::preset(preset).unwrap();
            let ladder = ladder(&s, None, true).unwrap();
            let atlas = ladder.twig.as_ref().unwrap();
            assert_eq!(ladder.levels.len(), 4);
            for (level, budget) in ladder.levels.iter().zip(BUDGETS) {
                assert!(level.triangles() <= budget, "{preset} {}: {} > {budget}", level.rung.name, level.triangles());
                assert!(level.leaves.v.iter().all(|v| v.p.is_finite() && v.n.is_finite() && (0. ..=1.).contains(&v.uv.x)));
            }
            // near and mid spend their budgets (the old ladder landed at ~60 %)
            for level in &ladder.levels[..2] {
                assert!(level.triangles() * 10 >= level.rung.budget * 9, "{preset} {}: {} of {}", level.rung.name, level.triangles(), level.rung.budget);
            }
            let covered = atlas.color.chunks_exact(4).filter(|p| p[3] > 128).count();
            assert!(covered > (EDGE * EDGE / 20) as usize, "{preset}: atlas almost empty");
            assert!(atlas.normal.chunks_exact(4).all(|p| p[0] == 0));
        }
    }

    #[test]
    fn principal_axes_find_a_flat_spray() {
        let points: Vec<Vec3> = (0..50).map(|k| { let a = k as f32 * 0.7; Vec3::new(a.sin() * 2., (a * 1.3).cos(), 0.01 * a.cos()) }).collect();
        let (values, axes) = principal_axes(&points, Vec3::ZERO);
        assert!(values[0] >= values[1] && values[1] >= values[2]);
        assert!(axes[2].z.abs() > 0.99);
        assert!(axes[0].x.abs() > 0.99);
    }
}
