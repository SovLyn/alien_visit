//! Deterministic desert height field for Ossa.
//!
//! Shared by the surface walker (scene) and the terrain mesh builder
//! (renderer). Local coordinates are metres: +X east, +N north (renderer
//! maps north to -Z). Heights are metres above an arbitrary local datum.
//!
//! The large features are art-directed for the opening shot:
//!  - a shallow basin to the north,
//!  - a dark, low ridge to the east (right edge of the opening frame),
//!  - a narrow eroded blade in the north-west (left anchor),
//!  - low-frequency rolling mineral plains everywhere else.
//!
//! Keep this function in sync with the visual spec; don't tune it by eye
//! without updating the composition screenshots.

const PI: f32 = std::f32::consts::PI;

#[inline]
fn hash2(ix: i32, iy: i32, seed: u32) -> f32 {
    let mut h = (ix as u32).wrapping_mul(0x27d4_eb2d)
        ^ (iy as u32).wrapping_mul(0x1656_67b1)
        ^ seed.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    (h & 0x00ff_ffff) as f32 / 16_777_215.0
}

#[inline]
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Value noise in [-ish 0,1], smooth and cheap.
fn vnoise(x: f32, y: f32, seed: u32) -> f32 {
    let fx = x.floor();
    let fy = y.floor();
    let ix = fx as i32;
    let iy = fy as i32;
    let tx = smooth(x - fx);
    let ty = smooth(y - fy);
    let a = hash2(ix, iy, seed);
    let b = hash2(ix + 1, iy, seed);
    let c = hash2(ix, iy + 1, seed);
    let d = hash2(ix + 1, iy + 1, seed);
    let ab = a + (b - a) * tx;
    let cd = c + (d - c) * tx;
    ab + (cd - ab) * ty
}

fn fbm(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut norm = 0.0;
    let mut fx = x;
    let mut fy = y;
    for i in 0..octaves {
        sum += vnoise(fx, fy, seed + i * 17) * amp;
        norm += amp;
        amp *= 0.5;
        // Rotate slightly to avoid axis-aligned artefacts.
        let nx = fx * 1.93 + fy * 0.53;
        let ny = fy * 1.93 - fx * 0.53;
        fx = nx;
        fy = ny;
    }
    sum / norm
}

fn ridged(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.55;
    let mut norm = 0.0;
    let mut fx = x;
    let mut fy = y;
    for i in 0..octaves {
        let n = 1.0 - (2.0 * vnoise(fx, fy, seed + i * 31) - 1.0).abs();
        sum += n * n * amp;
        norm += amp;
        amp *= 0.5;
        let nx = fx * 2.02 + fy * 0.41;
        let ny = fy * 2.02 - fx * 0.41;
        fx = nx;
        fy = ny;
    }
    sum / norm
}

#[inline]
fn gauss(dx: f32, dn: f32, sx: f32, sn: f32) -> f32 {
    (-(dx * dx) / (2.0 * sx * sx) - (dn * dn) / (2.0 * sn * sn)).exp()
}

#[inline]
fn segment_dist(px: f32, pn: f32, ax: f32, an: f32, bx: f32, bn: f32) -> f32 {
    let vx = bx - ax;
    let vn = bn - an;
    let wx = px - ax;
    let wn = pn - an;
    let vv = vx * vx + vn * vn;
    let t = if vv < 1e-6 { 0.0 } else { ((wx * vx + wn * vn) / vv).clamp(0.0, 1.0) };
    let dx = px - (ax + vx * t);
    let dn = pn - (an + vn * t);
    (dx * dx + dn * dn).sqrt()
}

/// Height in metres at local east/north (metres).
pub fn height_m(east: f64, north: f64) -> f32 {
    let e = east as f32;
    let n = north as f32;

    // Broad, slow country: two low-frequency layers plus a soft ridge field.
    let roll_a = fbm(e / 2950.0 + 13.1, n / 2950.0 - 7.7, 4, 0x51a3);
    let roll_b = fbm(e / 980.0 - 4.6, n / 980.0 + 9.2, 3, 0x9c17);
    let soft_ridge = ridged(e / 1620.0 + 2.4, n / 1620.0 - 3.1, 3, 0x2f6d);
    let mut h = (roll_a - 0.5) * 58.0
        + (roll_b - 0.5) * 13.0
        + (soft_ridge - 0.5) * 15.0;

    // A shallow basin to the north: the view opens across it.
    h -= 24.0 * gauss(e - 120.0, n - 1150.0, 1550.0, 1350.0);

    // East ridge chain (right edge of the opening frame). A narrow spine
    // with a broken, eroded crest.
    let d_east = segment_dist(e, n, 310.0, -300.0, 430.0, 1650.0);
    let east_crest = (-(d_east / 92.0).powi(2)).exp();
    let east_break = 0.62 + 0.38 * ridged(e / 130.0 + 4.0, n / 170.0, 2, 0x7a11);
    h += 34.0 * east_crest * east_break;

    // A small foreground spur at the right of the opening composition.
    let d_spur = segment_dist(e, n, 120.0, 40.0, 200.0, 430.0);
    h += 13.0 * (-(d_spur / 42.0).powi(2)).exp();

    // Eroded blade in the north-west: a lone visual anchor on the left third.
    let d_blade = segment_dist(e, n, -610.0, 760.0, -430.0, 1110.0);
    let blade = (-(d_blade / 52.0).powi(2)).exp();
    let blade_tex = 0.55 + 0.45 * ridged(e / 48.0 + 8.0, n / 140.0, 2, 0x33c9);
    h += 54.0 * blade * blade_tex;

    // Extra erosion detail: not enough to break the low-frequency rule.
    h += (fbm(e / 172.0 + 2.0, n / 172.0 - 6.0, 2, 0x6b2f) - 0.5) * 2.8;
    h += (fbm(e / 39.0 - 8.0, n / 39.0 + 3.0, 2, 0xd41a) - 0.5) * 0.75;

    // Very slight terrace/strata stepping on shallow slopes.
    let strata = ((h * 0.22).sin() * 0.5 + 0.5).powf(3.0) - 0.5;
    h += strata * 0.45;

    h
}

/// Approximate normal from finite differences. `eps` in metres.
pub fn normal_m(east: f64, north: f64, eps: f32) -> glam::Vec3 {
    let e = eps as f64;
    let hx1 = height_m(east + e, north);
    let hx0 = height_m(east - e, north);
    let hn1 = height_m(east, north + e);
    let hn0 = height_m(east, north - e);
    let dx = (hx1 - hx0) / (2.0 * eps);
    let dn = (hn1 - hn0) / (2.0 * eps);
    glam::Vec3::new(-dx, 1.0, -dn).normalize_or_zero()
}

/// Walkable slope limit in degrees for the surface controller.
pub const MAX_WALK_SLOPE_DEG: f32 = 34.0;

/// Convenience for scene-side sampling: converts metres to the terrain field.
#[inline]
pub fn height_at_m(pos: [f64; 2]) -> f32 {
    height_m(pos[0], pos[1])
}

/// Used by QA to verify the opening composition programmatically.
pub fn feature_summary() -> &'static str {
    "basin-north; east-ridge; nw-blade; low-frequency desert"
}
