//! Baked twig cards (`--twig-cards`): a game LOD ladder that stays full at Reforger-style
//! budgets (8000 / 3000 / 900 / 250 P3D faces, the reversed face of each card triangle counted).
//!
//! At such budgets the classic ladder thins single leaves until the crown reads as a sapling.
//! Here the thin wood and its leaves are not drawn as geometry at all: every leafy twig (the
//! subtree under a branch thinner than a radius cutoff) becomes one or two alpha cards whose
//! texture is a real twig of the same tree, rendered orthographically into an atlas with its
//! colour, coverage and tangent-space normal. Thousands of leaves then cost a few triangles.
//!
//! | level    | wood                                | foliage                                       |
//! |----------|-------------------------------------|-----------------------------------------------|
//! | near     | branches above the cutoff           | twig cards (front + crossed side view)        |
//! | mid      | thicker cutoff, inside the cards    | branch cards baked from larger subtrees       |
//! | far      | trunk and limbs, inside the cards   | limb cards, 4 variants                        |
//! | very far | none (baked into the cards)         | 2 crossed trimmed 2x2 cards + 1 horizontal    |
//!
//! The cutoff of each level is searched so its cards fill the level's budget, and what the
//! cards leave is given to the wood. Past near the wood is clipped to the crown the cards draw
//! (no bare limbs). Each level's cards are then scaled until its projected coverage (side, 45
//! degrees, below, above) is within 3 % of the near level's, so density does not step.
//! Atlas: `EDGE` x `EDGE`, cells with an 8 texel transparent gutter (see `layout`).
use crate::gamelod::{self, Ladder, Level, Rung, LADDER};
use crate::tree::{Part, Settings, V};
use macroquad::prelude::{vec2, Vec2, Vec3};
use std::collections::HashMap;

pub const EDGE: u32 = 2048;
/// Supersampling per axis while baking.
const SS: u32 = 3;
/// The default budgets of `--twig-cards`, in P3D faces as the engine draws them: every card
/// triangle counts twice (its reversed face), wood once.
pub const BUDGETS: [usize; 4] = [8_000, 3_000, 900, 250];

/// The baked atlas: `color` is sRGB RGBA with coverage in alpha; `normal` is the tangent-space
/// normal in OpenPoseidon's `_nohq` layout (R 0, G y, B z, A x), X along +U and Y along -V
/// (up in the image), as the bark normal maps.
pub struct TwigAtlas {
    pub edge: u32,
    pub color: Vec<u8>,
    pub normal: Vec<u8>,
    /// Debug: per level, how many groups (cards' subtrees) and the radius cutoff used, and the
    /// projected coverage per level.
    pub report: Vec<serde_json::Value>,
    /// Every cell (x, y, w, h in texels), for offline tools; each is a whole number of the
    /// 256 texel tiles the masked mip chain keeps alpha-test coverage in.
    pub cells: Vec<[u32; 4]>,
}

/// An atlas cell in texels. The outer `GUTTER` texels stay transparent (colour bled into them),
/// so mipmaps and BC blocks of neighbouring cells do not bleed into each other.
#[derive(Clone, Copy)]
struct Cell {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

const GUTTER: u32 = 8;

impl Cell {
    fn square(x: u32, y: u32, size: u32) -> Cell {
        Cell { x, y, w: size, h: size }
    }
    /// UV of the point (s, t) in 0..1 (t down) of the cell's inner area.
    fn uv(&self, s: f32, t: f32) -> Vec2 {
        vec2((self.x as f32 + GUTTER as f32 + s * (self.w - 2 * GUTTER) as f32) / EDGE as f32,
            (self.y as f32 + GUTTER as f32 + t * (self.h - 2 * GUTTER) as f32) / EDGE as f32)
    }
}

/// The atlas layout: near 4 twigs x 2 views (512), mid 2 x 2 (512), far 4 x 2 (256), very far
/// front and side (256 x 512) and top (512).
fn layout() -> [Vec<Cell>; 4] {
    let near = (0..8).map(|k| Cell::square(k % 4 * 512, k / 4 * 512, 512)).collect();
    let mid = (0..4).map(|k| Cell::square(k * 512, 1024, 512)).collect();
    let far = (0..8).map(|k| Cell::square(k % 4 * 256, 1536 + k / 4 * 256, 256)).collect();
    let very_far = vec![Cell { x: 1024, y: 1536, w: 256, h: 512 }, Cell { x: 1280, y: 1536, w: 256, h: 512 },
        Cell::square(1536, 1536, 512)];
    [near, mid, far, very_far]
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

/// Renders the triangles of a subtree in `view` into the inner area of `cell`.
fn bake(tree: &Tree, view: &View, leaf_tris: &[u32], bark_tris: &[u32], atlas: &mut TwigAtlas, cell: Cell) {
    let (iw, ih) = ((cell.w - 2 * GUTTER) as usize, (cell.h - 2 * GUTTER) as usize);
    let (sw, sh) = (iw * SS as usize, ih * SS as usize);
    let mut depth = vec![f32::MIN; sw * sh];
    let mut color = vec![[0f32; 3]; sw * sh];
    let mut normal = vec![Vec3::ZERO; sw * sh];
    let zr = (view.z.0, (view.z.1 - view.z.0).max(1e-4));
    let project = |p: Vec3| {
        let l = view.local(p);
        (vec2((l.x - view.x.0) / (view.x.1 - view.x.0) * sw as f32, (view.y.1 - l.y) / (view.y.1 - view.y.0) * sh as f32), l.z)
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
                if px >= sw || py >= sh { return; }
                let i = py * sw + px;
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
                    // a species' young shoots carry their own colour (`twigs color=`)
                    if vs[0].c[..3] != [1., 1., 1.] {
                        let c = vs[0].c;
                        let lum = [0, 1, 2].map(|ch| gamelod::linear(px[si + ch])).iter().sum::<f32>() / 3.;
                        let shade = (lum / 0.2).clamp(0.5, 1.5);
                        [c[0] * shade, c[1] * shade, c[2] * shade]
                    } else {
                        [0, 1, 2].map(|ch| gamelod::linear(px[si + ch]))
                    }
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
    let (cw, ch) = (cell.w as usize, cell.h as usize);
    let g = GUTTER as usize;
    let mut texels = vec![None::<([f32; 3], Vec3)>; cw * ch];
    for ty in 0..ch {
        for tx in 0..cw {
            let o = ((cell.y as usize + ty) * edge + cell.x as usize + tx) * 4;
            atlas.color[o + 3] = 0;
            if tx < g || ty < g || tx >= g + iw || ty >= g + ih { continue; }
            let (ix, iy) = (tx - g, ty - g);
            let (mut sum, mut nsum, mut k) = ([0f32; 3], Vec3::ZERO, 0);
            for sy in 0..SS as usize {
                for sx in 0..SS as usize {
                    let i = (iy * SS as usize + sy) * sw + ix * SS as usize + sx;
                    if depth[i] > f32::MIN {
                        k += 1;
                        for c in 0..3 { sum[c] += color[i][c]; }
                        nsum += normal[i];
                    }
                }
            }
            atlas.color[o + 3] = (k as f32 / (SS * SS) as f32 * 255.).round() as u8;
            if k > 0 {
                texels[ty * cw + tx] = Some((sum.map(|x| x / k as f32), nsum.normalize_or(Vec3::Z)));
            }
        }
    }
    // Bleed colour and normal into the empty texels and the gutter, so mipmaps and BC blocks do
    // not pull in black at the outline.
    for _ in 0..16 {
        let previous = texels.clone();
        for y in 0..ch {
            for x in 0..cw {
                if previous[y * cw + x].is_some() { continue; }
                let (mut sum, mut nsum, mut k) = ([0f32; 3], Vec3::ZERO, 0);
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx < 0 || ny < 0 || nx >= cw as i32 || ny >= ch as i32 { continue; }
                    if let Some((c, n)) = previous[ny as usize * cw + nx as usize] {
                        k += 1;
                        for c2 in 0..3 { sum[c2] += c[c2]; }
                        nsum += n;
                    }
                }
                if k > 0 { texels[y * cw + x] = Some((sum.map(|s| s / k as f32), nsum.normalize_or(Vec3::Z))); }
            }
        }
    }
    let fallback = texels.iter().flatten().next().map_or([0.05, 0.1, 0.03], |t| t.0);
    for ty in 0..ch {
        for tx in 0..cw {
            let o = ((cell.y as usize + ty) * edge + cell.x as usize + tx) * 4;
            let (c, n) = texels[ty * cw + tx].unwrap_or((fallback, Vec3::Z));
            for k in 0..3 { atlas.color[o + k] = gamelod::srgb_byte(c[k]); }
            let n = Vec3::new(n.x, n.y, n.z.max(0.05)).normalize();
            let byte = |c: f32| ((c * 0.5 + 0.5) * 255.).round().clamp(0., 255.) as u8;
            atlas.normal[o..o + 4].copy_from_slice(&[0, byte(n.y), byte(n.z), byte(n.x)]);
        }
    }
}

/// Clips a polygon (cell coordinates) to the rectangle s0..s1 x t0..t1.
fn clip_rect(mut poly: Vec<Vec2>, s: (f32, f32), t: (f32, f32)) -> Vec<Vec2> {
    for (axis, limit, keep_below) in [(0, s.0, false), (0, s.1, true), (1, t.0, false), (1, t.1, true)] {
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
    if poly.len() > 1 && poly[0].distance_squared(poly[poly.len() - 1]) < 1e-10 { poly.pop(); }
    poly
}

/// The card outline of a baked cell in cell coordinates (0..1 over the inner area, t down): the
/// trimmed hull, or the whole cell.
fn outline(atlas: &TwigAtlas, cell: Cell, trim: bool) -> Vec<Vec2> {
    let full = vec![vec2(0., 1.), vec2(1., 1.), vec2(1., 0.), vec2(0., 0.)];
    if !trim { return full; }
    let edge = atlas.edge;
    let (x0, y0) = (cell.x + GUTTER, cell.y + GUTTER);
    let (iw, ih) = ((cell.w - 2 * GUTTER) as f32, (cell.h - 2 * GUTTER) as f32);
    let hull = gamelod::alpha_outline(&|x, y| atlas.color[((y * edge + x) * 4 + 3) as usize] >= 96,
        (x0, cell.x + cell.w - GUTTER), (y0, cell.y + cell.h - GUTTER), edge, 2.);
    if hull.len() < 3 { return full; }
    let poly = hull.iter().map(|p| vec2((p.x * edge as f32 - x0 as f32) / iw, (p.y * edge as f32 - y0 as f32) / ih)).collect();
    let poly = clip_rect(poly, (0., 1.), (0., 1.));
    if poly.len() >= 3 { poly } else { full }
}

/// Appends one card: `shape` (cell coordinates) on `view`'s plane, UVs into `cell`. Normals are
/// the card's, bent by `bend` toward the crown ellipsoid.
fn emit(out: &mut Part, tree: &Tree, view: &View, cell: Cell, shape: &[Vec2], owner: u32, bend: f32) {
    if shape.len() < 3 { return; }
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

/// The wood triangles (index triples into `bark`) that stay inside the crown the cards draw:
/// per height band and compass sector, no farther from the trunk axis than a card reaches.
/// Below the cards (the bole) everything stays; above them nothing.
fn clip_to_cards(indices: Vec<u32>, bark: &Part, cards: &Part) -> Vec<u32> {
    if cards.v.is_empty() { return indices; }
    const BANDS: usize = 24;
    const SECTORS: usize = 12;
    let (lo, hi) = cards.v.iter().fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(v.p.y), b.max(v.p.y)));
    let span = (hi - lo).max(1e-3);
    let cell = |p: Vec3| {
        let band = (((p.y - lo) / span) * BANDS as f32).floor().clamp(0., BANDS as f32 - 1.) as usize;
        let angle = p.z.atan2(p.x) + std::f32::consts::PI;
        let sector = ((angle / std::f32::consts::TAU) * SECTORS as f32).floor().clamp(0., SECTORS as f32 - 1.) as usize;
        (band, sector)
    };
    let mut reach = [[0f32; SECTORS]; BANDS];
    for v in &cards.v {
        let (b, s) = cell(v.p);
        reach[b][s] = reach[b][s].max(vec2(v.p.x, v.p.z).length());
    }
    // neighbours share their reach, so a gap between two cards does not cut a limb in half
    let mut smooth = reach;
    for b in 0..BANDS {
        for s in 0..SECTORS {
            for db in [-1i32, 0, 1] {
                for ds in [-1i32, 0, 1] {
                    let nb = b as i32 + db;
                    if nb < 0 || nb >= BANDS as i32 { continue; }
                    let ns = (s as i32 + ds).rem_euclid(SECTORS as i32) as usize;
                    smooth[b][s] = smooth[b][s].max(reach[nb as usize][ns]);
                }
            }
        }
    }
    indices.chunks_exact(3).filter(|t| {
        let c = t.iter().map(|&i| bark.v[i as usize].p).sum::<Vec3>() / 3.;
        if c.y < lo { return true; }
        if c.y > hi { return false; }
        let (b, s) = cell(c);
        vec2(c.x, c.z).length() <= smooth[b][s]
    }).flatten().copied().collect()
}

/// The four directions the projected coverage is measured from: side, 45 degrees round, from
/// below and from above (image right `t`, image up `b`).
const COVERAGE_VIEWS: [(&str, [f32; 3], [f32; 3]); 4] = [
    ("side", [1., 0., 0.], [0., 1., 0.]),
    ("45deg", [std::f32::consts::FRAC_1_SQRT_2, 0., -std::f32::consts::FRAC_1_SQRT_2], [0., 1., 0.]),
    ("below", [1., 0., 0.], [0., 0., 1.]),
    ("above", [1., 0., 0.], [0., 0., -1.]),
];

/// Projected area (m^2) a level covers from each of `COVERAGE_VIEWS`: wood opaque, cards where
/// the atlas alpha passes the 0.5 test, at `px` metres per pixel.
fn coverage(level: &Level, atlas: &TwigAtlas, px: f32) -> [f32; 4] {
    COVERAGE_VIEWS.map(|(_, t, b)| {
        let (t, b) = (Vec3::from_array(t), Vec3::from_array(b));
        let points = level.bark.v.iter().chain(&level.leaves.v).map(|v| vec2(v.p.dot(t), v.p.dot(b)));
        let (lo, hi) = points.fold((Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)), |(a, c), p| (a.min(p), c.max(p)));
        if lo.x > hi.x { return 0.; }
        let (w, h) = (((hi.x - lo.x) / px) as usize + 2, ((hi.y - lo.y) / px) as usize + 2);
        let mut hit = vec![false; w * h];
        let project = |p: Vec3| (vec2((p.dot(t) - lo.x) / px, (hi.y - p.dot(b)) / px), 0.);
        for (part, cards) in [(&level.bark, false), (&level.leaves, true)] {
            for tri in part.i.chunks_exact(3) {
                let vs: [V; 3] = std::array::from_fn(|k| part.v[tri[k] as usize]);
                let [a, bb, c] = vs.map(|v| project(v.p));
                gamelod::raster(a, bb, c, |x, y, l0, l1, l2| {
                    if x >= w || y >= h { return; }
                    if cards {
                        let uv = vs[0].uv * l0 + vs[1].uv * l1 + vs[2].uv * l2;
                        let ax = ((uv.x * atlas.edge as f32) as u32).min(atlas.edge - 1);
                        let ay = ((uv.y * atlas.edge as f32) as u32).min(atlas.edge - 1);
                        if atlas.color[((ay * atlas.edge + ax) * 4 + 3) as usize] < 128 { return; }
                    }
                    hit[y * w + x] = true;
                });
            }
        }
        hit.iter().filter(|&&h| h).count() as f32 * px * px
    })
}

/// Grows or shrinks every card of `leaves` (vertex ranges `cards`) about its centre, by `scale`
/// across (x, z) and along y, from the positions `original`. `grounded`: a card keeps its lowest
/// point (the whole-tree cards stand on the ground).
fn scale_cards(leaves: &mut Part, original: &[V], cards: &[(usize, usize)], scale: Vec2, grounded: bool) {
    let s = Vec3::new(scale.x, scale.y, scale.x);
    for &(a, b) in cards {
        let mut centre = original[a..b].iter().map(|v| v.p).sum::<Vec3>() / (b - a) as f32;
        if grounded { centre.y = original[a..b].iter().map(|v| v.p.y).fold(f32::MAX, f32::min); }
        for k in a..b { leaves.v[k].p = centre + (original[k].p - centre) * s; }
    }
}

/// The default `--card-bend`: OpenPoseidon bends foliage normals toward the canopy itself, so
/// the cards keep their own normals (the bend is applied once, by the engine).
pub const DEFAULT_BEND: f32 = 0.0;

/// The `--twig-cards` ladder. `budgets` are P3D faces per level, the reversed face of every card
/// triangle included (default `BUDGETS`); `bend` is the share of the crown ellipsoid normal in the
/// card vertex normals (`--card-bend`, default `DEFAULT_BEND`).
pub fn ladder(s: &Settings, budgets: Option<&[usize]>, trim: bool, bend: f32) -> Result<Ladder, String> {
    let budgets: Vec<usize> = budgets.map_or(BUDGETS.to_vec(), |b| b.to_vec());
    if budgets.len() != 4 || budgets.windows(2).any(|w| w[1] >= w[0]) || budgets[3] < 48 {
        return Err("--twig-cards --budgets wants four shrinking P3D face counts, the last at least 48".into());
    }
    if !(0. ..=1.).contains(&bend) {
        return Err("--card-bend wants 0..1".into());
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
    let cells = layout();
    let mut atlas = TwigAtlas { edge: EDGE, color: vec![0; edge * edge * 4], normal: vec![0; edge * edge * 4], report: Vec::new(),
        cells: cells.iter().flatten().map(|c| [c.x, c.y, c.w, c.h]).collect() };
    for p in atlas.normal.chunks_exact_mut(4) { p.copy_from_slice(&[0, 128, 255, 128]); }
    // (variants, views) of near, mid, far
    let specs = [(4usize, 2usize), (2, 2), (4, 2)];
    // every radius a cutoff can sit at, thinnest first
    let mut radii: Vec<f32> = tree.radius.clone();
    radii.push(f32::MAX);
    radii.sort_by(f32::total_cmp);
    radii.dedup();
    let mut levels = Vec::new();
    let mut card_ranges: Vec<Vec<(usize, usize)>> = Vec::new();
    let mut previous_cutoff = 0f32;
    for (k, &(variants, views)) in specs.iter().enumerate() {
        let mut rung: Rung = LADDER[k];
        rung.budget = budgets[k];
        rung.min_branch = 0.;
        // P3D faces: an untrimmed card is 2 triangles, a trimmed one at most 6 (usually 4-5),
        // each drawn twice (the reversed face)
        let faces_per_group = views * if trim { 12 } else { 4 };
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
        let quantiles: &[f32] = match variants { 1 => &[0.75], 2 => &[0.55, 0.85], _ => &[0.4, 0.6, 0.75, 0.9] };
        let mut reps: Vec<usize> = quantiles.iter().filter_map(|q| candidates.get(((candidates.len() as f32 - 1.) * q).round() as usize).map(|c| c.0)).collect();
        reps.dedup();
        let mut shapes = Vec::new();
        for (r, &g) in reps.iter().enumerate() {
            let group_views = tree.views(&groups[g]);
            for (v, view) in group_views.iter().enumerate().take(views) {
                let cell = cells[k][r * views + v];
                bake(&tree, view, &groups[g].leaf_tris, &groups[g].bark_tris, &mut atlas, cell);
                shapes.push(outline(&atlas, cell, trim));
            }
        }
        let mut leaves = Part::default();
        let mut ranges = Vec::new();
        if !reps.is_empty() {
            for (g, group) in groups.iter().enumerate() {
                let (_, size, aspect) = sizes[g];
                let r = (0..reps.len()).min_by(|&a, &b| {
                    let score = |r: usize| { let (_, s, asp) = sizes[reps[r]]; (size / s).ln().abs() + (aspect / asp).ln().abs() };
                    score(a).total_cmp(&score(b))
                }).unwrap();
                for (v, view) in tree.views(group).iter().enumerate().take(views) {
                    let slot = r * views + v;
                    let start = leaves.v.len();
                    emit(&mut leaves, &tree, view, cells[k][slot], &shapes[slot], group.owner, bend);
                    if leaves.v.len() > start { ranges.push((start, leaves.v.len())); }
                }
            }
        }
        // the wood that stays gets whatever the cards leave; past near only what the cards cover
        let mut kept: Vec<u32> = (0..nb).filter(|&b| wood[b]).flat_map(|b| tree.bark_by_branch[b].iter().flat_map(|&t| (0..3).map(move |j| t * 3 + j))).map(|i| full.bark.i[i as usize]).collect();
        if k > 0 { kept = clip_to_cards(kept, &full.bark, &leaves); }
        let kept = Part { v: full.bark.v.clone(), i: kept };
        let mut bark = Part::default();
        let card_faces = leaves.i.len() * 2 / 3;
        let mut left = rung.budget.saturating_sub(card_faces);
        for _ in 0..3 {
            rung.wood = left.max(12);
            bark = gamelod::simplify_wood(&kept, &rung, height);
            let total = bark.i.len() / 3 + card_faces;
            if total <= rung.budget { break; }
            left = left.saturating_sub(total - rung.budget);
        }
        atlas.report.push(serde_json::json!({"level": rung.name, "cutoffRadius": cutoff, "groups": groups.len(),
            "twigGroups": groups.iter().filter(|g| g.twig).count(), "variants": reps.len(),
            "cardTriangles": leaves.i.len() / 3, "woodTriangles": bark.i.len() / 3}));
        levels.push(Level { rung, bark, leaves });
        card_ranges.push(ranges);
    }
    // very far: the whole tree on two crossed cards through the trunk (trimmed, 2 x 2 grid) and
    // one horizontal card through the crown, seen from above
    let mut rung: Rung = LADDER[3];
    rung.budget = budgets[3];
    let all_leaves: Vec<u32> = (0..full.leaves.i.len() as u32 / 3).collect();
    let all_bark: Vec<u32> = (0..full.bark.i.len() as u32 / 3).collect();
    let points: Vec<Vec3> = full.bark.v.iter().chain(&full.leaves.v).map(|v| v.p).collect();
    let crown_points: Vec<Vec3> = full.leaves.v.iter().map(|v| v.p).collect();
    let mut leaves = Part::default();
    let mut ranges = Vec::new();
    for (v, (t, b)) in [(Vec3::X, Vec3::Y), (-Vec3::Z, Vec3::Y), (Vec3::X, -Vec3::Z)].into_iter().enumerate() {
        let top = v == 2;
        let bake_view = View::fit(Vec3::ZERO, t, b, if top { &crown_points } else { &points });
        let cell = cells[3][v];
        bake(&tree, &bake_view, &all_leaves, if top { &[] } else { &all_bark }, &mut atlas, cell);
        let shape = outline(&atlas, cell, true);
        let mut view = bake_view;
        // vertical cards through the trunk axis; the horizontal one at the crown's middle
        view.z = if top { (crown.y, crown.y) } else { (0., 0.) };
        let start = leaves.v.len();
        let pieces: Vec<Vec<Vec2>> = if top { vec![shape] } else {
            (0..4).map(|q| clip_rect(shape.clone(), ((q % 2) as f32 * 0.5, (q % 2 + 1) as f32 * 0.5),
                ((q / 2) as f32 * 0.5, (q / 2 + 1) as f32 * 0.5))).collect()
        };
        for piece in pieces { emit(&mut leaves, &tree, &view, cell, &piece, 0, bend); }
        // a card scales as a whole, or its grid pieces would part
        if leaves.v.len() > start { ranges.push((start, leaves.v.len())); }
    }
    atlas.report.push(serde_json::json!({"level": rung.name, "cards": 3, "cardTriangles": leaves.i.len() / 3}));
    levels.push(Level { rung, bark: Part::default(), leaves });
    card_ranges.push(ranges);
    // Equal projected coverage on every level, so density does not step at the switches (a
    // forest would show rings): grow or shrink each level's cards, across and along the height
    // separately, until each of the four views is within 4 % of the near level's.
    let px = height / 320.;
    let near = coverage(&levels[0], &atlas, px);
    let mut matched = vec![serde_json::json!({"level": levels[0].rung.name, "scale": [1.0, 1.0],
        "coverageM2": near, "ratio": [1.0, 1.0, 1.0, 1.0]})];
    for k in 1..levels.len() {
        let original = levels[k].leaves.v.clone();
        let mut scale = Vec2::ONE;
        let mut best = (f32::MAX, scale, [1f32; 4]);
        // very far: the two standing cards answer the side views, the horizontal one the views
        // from below and above, so each gets its own width
        let (standing, lying) = if k == 3 { card_ranges[k].split_at(2) } else { (&card_ranges[k][..], &[][..]) };
        let mut top = 1f32;
        let mut best_top = 1f32;
        for _ in 0..16 {
            scale_cards(&mut levels[k].leaves, &original, standing, scale, k == 3);
            scale_cards(&mut levels[k].leaves, &original, lying, Vec2::new(top, 1.), false);
            let c = coverage(&levels[k], &atlas, px);
            let ratios: [f32; 4] = std::array::from_fn(|d| if near[d] > 0. { c[d] / near[d] } else { 1. });
            let worst = ratios.iter().map(|r| (r - 1.).abs()).fold(0., f32::max);
            if worst < best.0 { best = (worst, scale, ratios); best_top = top; }
            if worst < 0.04 { break; }
            // from above and below the cards' spread across counts; from the side both
            let (side, vertical) = ((ratios[0] + ratios[1]) * 0.5, (ratios[2] + ratios[3]) * 0.5);
            if k == 3 {
                scale.x = (scale.x / side).clamp(0.6, 1.6);
                top = (top / vertical.sqrt()).clamp(0.6, 1.6);
                continue;
            }
            let across = (1. / vertical).sqrt().clamp(0.85, 1.18);
            // never taller than the tree: the silhouette stays, height only shrinks
            scale.x = (scale.x * across).clamp(0.6, 1.4);
            scale.y = (scale.y * (1. / (side * across)).clamp(0.85, 1.18)).clamp(0.6, 1.0);
        }
        let (_, scale, ratios) = best;
        let top = best_top;
        scale_cards(&mut levels[k].leaves, &original, standing, scale, k == 3);
        scale_cards(&mut levels[k].leaves, &original, lying, Vec2::new(top, 1.), false);
        let c = coverage(&levels[k], &atlas, px);
        matched.push(serde_json::json!({"level": levels[k].rung.name, "scale": [scale.x, scale.y, top], "coverageM2": c, "ratio": ratios}));
    }
    atlas.report.push(serde_json::json!({"coverage": matched, "views": COVERAGE_VIEWS.map(|v| v.0)}));
    Ok(Ladder { levels, atlas: Vec::new(), branches: full.branches.clone(), twig: Some(atlas) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twig_card_ladders_fill_but_never_exceed_their_budgets() {
        for preset in ["solstice-oak", "solstice-fir"] {
            let s = Settings::preset(preset).unwrap();
            let ladder = ladder(&s, None, true, DEFAULT_BEND).unwrap();
            let atlas = ladder.twig.as_ref().unwrap();
            assert_eq!(ladder.levels.len(), 4);
            for (level, budget) in ladder.levels.iter().zip(BUDGETS) {
                assert!(level.p3d_faces() <= budget, "{preset} {}: {} > {budget}", level.rung.name, level.p3d_faces());
                assert!(level.leaves.v.iter().all(|v| v.p.is_finite() && v.n.is_finite() && (0. ..=1.).contains(&v.uv.x)));
            }
            // near and mid spend their budgets (the old ladder landed at ~60 %)
            for level in &ladder.levels[..2] {
                assert!(level.p3d_faces() * 10 >= level.rung.budget * 9, "{preset} {}: {} of {}", level.rung.name, level.p3d_faces(), level.rung.budget);
            }
            let covered = atlas.color.chunks_exact(4).filter(|p| p[3] > 128).count();
            assert!(covered > (EDGE * EDGE / 20) as usize, "{preset}: atlas almost empty");
            assert!(atlas.normal.chunks_exact(4).all(|p| p[0] == 0));
            // the gutters stay transparent
            for c in &atlas.cells {
                for x in c[0]..c[0] + c[2] {
                    assert_eq!(atlas.color[((c[1] * EDGE + x) * 4 + 3) as usize], 0);
                }
            }
            // every level covers about what near covers
            let px = 0.05;
            let near = coverage(&ladder.levels[0], atlas, px).iter().sum::<f32>();
            for level in &ladder.levels[1..] {
                let c = coverage(level, atlas, px).iter().sum::<f32>();
                assert!((c / near - 1.).abs() < 0.12, "{preset} {}: coverage {c} vs near {near}", level.rung.name);
            }
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

    #[test]
    fn rect_clipping_keeps_the_inside() {
        let square = vec![vec2(0., 0.), vec2(1., 0.), vec2(1., 1.), vec2(0., 1.)];
        let q = clip_rect(square, (0.5, 1.), (0., 0.5));
        assert_eq!(q.len(), 4);
        let area: f32 = (0..q.len()).map(|i| { let (a, b) = (q[i], q[(i + 1) % q.len()]); a.x * b.y - b.x * a.y }).sum::<f32>().abs() * 0.5;
        assert!((area - 0.25).abs() < 1e-5);
    }
}
