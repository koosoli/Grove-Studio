//! Alpha-tested mip chains, ported from Solstice (SamG-Coder, MIT,
//! `crates/solstice-builder/src/texture.rs` `masked_mip_chain`, revision f214c0e).
//!
//! Colour is averaged by alpha and empty texels take the colour of the level above, so a
//! card's background never bleeds into its edge; each tile keeps the base level's alpha-test
//! coverage by moving its threshold (Castano 2010). Grove change: 8 tiles per side instead of
//! 4, so on the 2048 twig atlas a tile is one 256 texel cell (every twig-card cell is a whole
//! number of them). This is the only coverage lift for the twig atlas: OpenPoseidon's
//! `cutout_mip_alpha` must stay off for it (its shadow pass applies none, this one reaches it).

const COVERAGE_TILES: usize = 8;
/// The least edge of such a tile, in texels: with fewer, a tile's texels are too few to hold a
/// fraction of coverage, and thin detail would round away.
const MIN_TILE_EDGE: usize = 8;

/// How many tiles per side a level of this edge is split into.
fn tiles_for(edge: usize) -> usize {
    (edge / MIN_TILE_EDGE).clamp(1, COVERAGE_TILES)
}

/// Moves the point where `alpha` crosses 0.5 from `threshold` to 128 (of 255), so that what passes
/// a test at `threshold` is exactly what passes a test at 0.5 afterwards. 255 stays where it is,
/// and so does 0, unless a `threshold` of 0 means everything passes. A `threshold` of 256 passes
/// nothing.
fn move_alpha_threshold(alpha: u8, threshold: u16) -> u8 {
    let (alpha, threshold) = (u32::from(alpha), u32::from(threshold));
    if alpha < threshold {
        (alpha * 128 / threshold) as u8
    } else if threshold >= 255 {
        255
    } else {
        (128 + (alpha - threshold) * 127 / (255 - threshold)) as u8
    }
}

/// Mip chain of an RGBA8 image whose alpha is a cut-out mask at 0.5.
///
/// * Alpha keeps its coverage (Castaño 2010): a plain box filter pulls thin strands below 0.5 and
///   they vanish with distance, so each level moves its own threshold until as much of it passes
///   as passes in the largest level. This is done per tile, as an atlas's cells differ, with ties
///   parted by a fixed pattern in the texture, so it holds still under the camera.
/// * Colour is averaged by alpha, and texels with no alpha at all take the colour of the level
///   above, so bilinear filtering at an edge never reaches a texel's unrelated colour.
pub fn masked_mip_chain(base: Vec<u8>, size: u32) -> Vec<Vec<u8>> {
    let mut levels = alpha_weighted_pyramid(base, size);
    fill_transparent_colour(&mut levels);

    let targets = passing_per_tile(&levels[0], size as usize);
    let mut edge = size as usize;
    for level in &mut levels[1..] {
        edge /= 2;
        keep_coverage(level, edge, size as usize, &targets);
    }
    levels
}

/// Halves an RGBA8 image down to 1x1, with alpha box-filtered and colour weighted by alpha.
fn alpha_weighted_pyramid(base: Vec<u8>, size: u32) -> Vec<Vec<u8>> {
    let mut levels = vec![base];
    let mut edge = size as usize;
    while edge > 1 {
        let previous = levels.last().expect("chain starts non-empty");
        let half = edge / 2;
        let mut next = Vec::with_capacity(half * half * 4);
        for y in 0..half {
            for x in 0..half {
                let mut colour = [0u32; 3];
                let mut alpha = 0u32;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let texel = &previous[((y * 2 + dy) * edge + x * 2 + dx) * 4..][..4];
                    let weight = u32::from(texel[3]);
                    alpha += weight;
                    for channel in 0..3 {
                        colour[channel] += u32::from(texel[channel]) * weight;
                    }
                }
                for value in colour {
                    next.push((value + alpha / 2).checked_div(alpha).unwrap_or(0) as u8);
                }
                next.push((alpha / 4) as u8);
            }
        }
        levels.push(next);
        edge = half;
    }
    levels
}

/// Gives every texel with no alpha the colour of the texel above it, from the top level down.
fn fill_transparent_colour(levels: &mut [Vec<u8>]) {
    for index in (0..levels.len() - 1).rev() {
        let (lower, upper) = levels.split_at_mut(index + 1);
        let (finer, coarser) = (&mut lower[index], &upper[0]);
        let edge = (finer.len() / 4).isqrt();
        for (i, texel) in finer.chunks_exact_mut(4).enumerate() {
            if texel[3] == 0 {
                let (x, y) = (i % edge / 2, i / edge / 2);
                texel[..3].copy_from_slice(&coarser[(y * (edge / 2) + x) * 4..][..3]);
            }
        }
    }
}

/// The tile of a `tiles`-per-side grid that a texel lies in.
fn tile_of(x: usize, y: usize, edge: usize, tiles: usize) -> usize {
    y * tiles / edge * tiles + x * tiles / edge
}

/// How many texels of each tile pass the test, and how many there are.
fn passing_per_tile(level: &[u8], edge: usize) -> Vec<(u64, u64)> {
    let tiles = tiles_for(edge);
    let mut counts = vec![(0, 0); tiles * tiles];
    for (i, texel) in level.chunks_exact(4).enumerate() {
        let count: &mut (u64, u64) = &mut counts[tile_of(i % edge, i / edge, edge, tiles)];
        count.0 += u64::from(texel[3] >= 128);
        count.1 += 1;
    }
    counts
}

/// Moves each tile's threshold so as much of `level` passes as `targets` say does in the largest
/// level, of edge `base_edge`.
fn keep_coverage(level: &mut [u8], edge: usize, base_edge: usize, targets: &[(u64, u64)]) {
    let base_tiles = tiles_for(base_edge);
    let tiles = tiles_for(edge);
    // A level too small for the whole grid merges its tiles.
    let mut wanted = vec![(0, 0); tiles * tiles];
    for (tile, count) in targets.iter().enumerate() {
        let (x, y) = (tile % base_tiles, tile / base_tiles);
        let merged: &mut (u64, u64) = &mut wanted[tile_of(x, y, base_tiles, tiles)];
        merged.0 += count.0;
        merged.1 += count.1;
    }

    let mut histograms = vec![vec![0u64; RANKS as usize]; tiles * tiles];
    for (i, texel) in level.chunks_exact(4).enumerate() {
        let (x, y) = (i % edge, i / edge);
        histograms[tile_of(x, y, edge, tiles)][rank(texel[3], x, y) as usize] += 1;
    }
    let thresholds: Vec<u32> = histograms
        .iter()
        .zip(&wanted)
        .map(|(histogram, &(passing, total))| {
            let texels: u64 = histogram.iter().sum();
            let wanted = (passing as f64 / total as f64 * texels as f64).round() as u64;
            rank_passing(histogram, wanted)
        })
        .collect();
    for (i, texel) in level.chunks_exact_mut(4).enumerate() {
        let (x, y) = (i % edge, i / edge);
        let threshold = thresholds[tile_of(x, y, edge, tiles)];
        let alpha = texel[3];
        let passes = rank(alpha, x, y) >= threshold;
        texel[3] = if u32::from(alpha) == threshold / RANKS_PER_ALPHA {
            // Texels parted only by the pattern would all filter to about 0.5, and so to nearly
            // nothing passing between them.
            if passes {
                255
            } else {
                0
            }
        } else {
            let moved = move_alpha_threshold(alpha, threshold.div_ceil(RANKS_PER_ALPHA) as u16);
            if passes {
                moved.max(128)
            } else {
                moved.min(127)
            }
        };
    }
}

/// Ranks each alpha value is split into, by the pattern in [`rank`].
const RANKS_PER_ALPHA: u32 = 16;
/// Ranks in all.
const RANKS: u32 = 256 * RANKS_PER_ALPHA;

/// Orders texels by alpha, and those of equal alpha by a fixed 4x4 pattern. A level of a fine
/// regular pattern, such as a net, can be one alpha throughout, where no threshold alone could
/// let some of it pass and not the rest: it would turn solid or vanish.
fn rank(alpha: u8, x: usize, y: usize) -> u32 {
    const BAYER: [[u32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    u32::from(alpha) * RANKS_PER_ALPHA + BAYER[y % 4][x % 4]
}

/// The rank that lets `wanted` texels of a histogram pass, as near as its ranks allow. Never one
/// of alpha 0: a texel with no alpha at all must stay a hole.
fn rank_passing(histogram: &[u64], wanted: u64) -> u32 {
    if wanted == 0 {
        return RANKS;
    }
    let mut passing = 0;
    for value in (RANKS_PER_ALPHA..RANKS).rev() {
        let with_this = passing + histogram[value as usize];
        if with_this >= wanted {
            // The nearer of stopping above this rank or below it.
            return if passing > 0 && wanted - passing <= with_this - wanted {
                value + 1
            } else {
                value
            };
        }
        passing = with_this;
    }
    RANKS_PER_ALPHA
}

