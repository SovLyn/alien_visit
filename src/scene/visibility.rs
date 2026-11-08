use crate::{frame::*, terrain_field};
use glam::Vec3;

pub fn ray_sphere(direction: Vec3, center: Vec3, radius: f32, limit: f32) -> bool {
    let direction = direction.as_dvec3().normalize();
    let center = center.as_dvec3();
    let radius = radius as f64;
    let limit = limit as f64;
    let along = center.dot(direction);
    if along <= 0. {
        return false;
    }
    // Cross-product distance avoids catastrophic f32 subtraction for a
    // small moon many millions of kilometres away.
    let perpendicular = center.cross(direction).length_squared();
    if perpendicular >= radius * radius {
        return false;
    }
    let entry = along - (radius * radius - perpendicular).sqrt();
    entry > 0. && entry < limit
}

/// Exact overlap of two circular apparent discs (angles in radians).
fn overlap(r: f64, s: f64, d: f64) -> f64 {
    if d >= r + s {
        return 0.;
    }
    if d <= (r - s).abs() {
        return std::f64::consts::PI * r.min(s).powi(2);
    }
    let a = ((d * d + r * r - s * s) / (2. * d * r))
        .clamp(-1., 1.)
        .acos();
    let b = ((d * d + s * s - r * r) / (2. * d * s))
        .clamp(-1., 1.)
        .acos();
    r * r * a + s * s * b
        - 0.5
            * ((-d + r + s) * (d + r - s) * (d - r + s) * (d + r + s))
                .max(0.)
                .sqrt()
}

pub fn sun_visibility(bodies: &[BodyView; MAX_BODIES]) -> f32 {
    let sun = bodies[BodyId::Kesh.index()];
    let p = sun.rel_pos_km.as_dvec3();
    let distance = p.length();
    let dir = p / distance;
    let radius = (sun.radius_km as f64 / distance).clamp(0., 1.).asin();
    let mut discs = [(0_f64, 0_f64, 0_f64); MAX_BODIES];
    let mut count = 0;
    let u = dir.cross(glam::DVec3::Y).normalize_or_zero();
    let v = dir.cross(u);
    for body in bodies {
        if !body.enabled || body.id == BodyId::Kesh {
            continue;
        }
        let b = body.rel_pos_km.as_dvec3();
        let dist = b.length();
        if dist >= distance || dist <= body.radius_km as f64 {
            continue;
        }
        let bdir = b / dist;
        let r = (body.radius_km as f64 / dist).clamp(0., 1.).asin();
        let separation = dir.dot(bdir).clamp(-1., 1.).acos();
        if separation >= radius + r {
            continue;
        }
        if r >= radius + separation {
            return 0.;
        }
        let tangent = bdir - dir * dir.dot(bdir);
        let length = tangent.length();
        let (x, y) = if length > 1e-14 {
            (
                tangent.dot(u) / length * separation,
                tangent.dot(v) / length * separation,
            )
        } else {
            (0., 0.)
        };
        discs[count] = (x, y, r);
        count += 1;
    }
    if count == 0 {
        return 1.;
    }
    let area = std::f64::consts::PI * radius * radius;
    if count == 1 {
        let (x, y, r) = discs[0];
        return (1. - overlap(radius, r, x.hypot(y)) / area).clamp(0., 1.) as f32;
    }
    // Rare simultaneous transits: integrate the UNION, never multiply eclipse
    // fractions (which double-counts overlapping occluders). Midpoint strips
    // bound the solar circle; interval union is allocation-free.
    let mut covered = 0.;
    for j in 0..256 {
        let y = radius * ((j as f64 + 0.5) / 128. - 1.);
        let half = (radius * radius - y * y).max(0.).sqrt();
        let mut intervals = [(0_f64, 0_f64); MAX_BODIES];
        let mut n = 0;
        for &(x, cy, r) in &discs[..count] {
            let square = r * r - (y - cy).powi(2);
            if square <= 0. {
                continue;
            }
            let dx = square.sqrt();
            let a = (x - dx).max(-half);
            let b = (x + dx).min(half);
            if b > a {
                intervals[n] = (a, b);
                n += 1;
            }
        }
        intervals[..n].sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut end = -half;
        for &(a, b) in &intervals[..n] {
            if b > end {
                covered += b - a.max(end);
                end = b;
            }
        }
    }
    (1. - covered * (radius / 128.) / area).clamp(0., 1.) as f32
}

pub fn terrain_blocked(direction: Vec3, position: [f64; 2], eye: f32, distance_km: f32) -> bool {
    let horizontal = direction.x.hypot(direction.z);
    if horizontal < 1e-6 {
        return direction.y < 0.;
    }
    let maximum = 30_000_f64.min(distance_km as f64 * 1000. * horizontal as f64);
    // Quadratic spacing: <6 m near the eye; <630 m at the far horizon.
    // 112 samples of exactly the same height function as the terrain mesh.
    for j in 1..=112 {
        let t = j as f64 / 112.;
        let d = 2. + (maximum - 2.).max(0.) * t * t;
        let east = position[0] + direction.x as f64 / horizontal as f64 * d;
        let north = position[1] - direction.z as f64 / horizontal as f64 * d;
        let ray = eye as f64 + direction.y as f64 / horizontal as f64 * d;
        if terrain_field::height_m(east, north) as f64 > ray + 0.08 {
            return true;
        }
    }
    false
}

pub fn marker_state(frame: &RenderFrame, id: BodyId) -> MarkerState {
    let body = frame.bodies[id.index()];
    if id == BodyId::Kesh && frame.sun_visibility < 0.9999 {
        return MarkerState::Eclipsed;
    }
    let distance = body.rel_pos_km.length();
    let dir = body.rel_pos_km / distance.max(1e-12);
    if frame.view == crate::settings::ViewMode::Surface
        && terrain_blocked(dir, frame.surface_pos_m, frame.eye_height_m, distance)
    {
        return MarkerState::Occluded;
    }
    for other in frame.bodies {
        if other.enabled
            && other.id != id
            && ray_sphere(
                dir,
                other.rel_pos_km,
                other.radius_km,
                distance - body.radius_km,
            )
        {
            return if id == BodyId::Kesh {
                MarkerState::Eclipsed
            } else {
                MarkerState::Occluded
            };
        }
    }
    MarkerState::Visible
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn circle_limits() {
        assert_eq!(overlap(1., 2., 4.), 0.);
        assert!((overlap(1., 2., 0.) - std::f64::consts::PI).abs() < 1e-12);
        assert!((overlap(1., 1., 1.) - (2. * (0.5_f64).acos() - 3_f64.sqrt() / 2.)).abs() < 1e-12);
    }
    #[test]
    fn eclipse_and_transit_fraction() {
        let empty = BodyView {
            id: BodyId::Kesh,
            rel_pos_km: Vec3::ZERO,
            radius_km: 0.,
            kind: BodyKind::Rock,
            axis: Vec3::Y,
            spin_rad: 0.,
            tint: Vec3::ONE,
            enabled: false,
        };
        let mut bodies = [empty; MAX_BODIES];
        bodies[0] = BodyView {
            id: BodyId::Kesh,
            rel_pos_km: Vec3::Z * 10_000.,
            radius_km: 100.,
            enabled: true,
            ..empty
        };
        bodies[1] = BodyView {
            id: BodyId::Vhal,
            rel_pos_km: Vec3::Z * 1000.,
            radius_km: 20.,
            enabled: true,
            ..empty
        };
        assert_eq!(sun_visibility(&bodies), 0.);
        bodies[1].radius_km = 5.;
        assert!((sun_visibility(&bodies) - 0.75).abs() < 0.0001);
        bodies[1].rel_pos_km.x = 100.;
        assert_eq!(sun_visibility(&bodies), 1.);
        bodies[1].rel_pos_km = Vec3::Z * 20_000.;
        bodies[1].radius_km = 1000.;
        assert_eq!(sun_visibility(&bodies), 1.); // Behind the source, no eclipse.
    }
    #[test]
    fn sphere_depth() {
        assert!(ray_sphere(Vec3::Z, Vec3::Z * 10., 2., 100.));
        assert!(!ray_sphere(Vec3::Z, Vec3::Z * 10., 2., 5.));
        assert!(!ray_sphere(Vec3::Z, -Vec3::Z * 10., 2., 100.));
    }
}
