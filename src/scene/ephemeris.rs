//! Double precision, hierarchical two-body ephemerides (km, seconds).
//! Epoch is solved from the *observer*, not from screen-space placements.
use crate::frame::{BodyId, MAX_BODIES};
use glam::{DQuat, DVec3};
use std::f64::consts::{PI, TAU};

pub const GM_KESH: f64 = 0.65 * 132_712_440_018.0;
pub const GM_VHAL: f64 = 0.90 * 126_686_534.0;
pub const GM_NADIR: f64 = 0.55 * 126_686_534.0;
pub const RADII: [f64; MAX_BODIES] = [140_000., 60_000., 1900., 620., 1050., 7500., 280., 3200.];
pub const LATITUDE: f64 = PI / 3.0;

#[derive(Clone, Copy, Debug)]
pub struct Elements {
    pub parent: BodyId,
    pub a: f64,
    pub e: f64,
    pub i: f64,
    pub node: f64,
    pub peri: f64,
    pub m0: f64,
    pub gm: f64,
}
impl Elements {
    fn new(parent: BodyId, a: f64, e: f64, i: f64, node: f64, peri: f64, m0: f64, gm: f64) -> Self {
        Self {
            parent,
            a,
            e,
            i: i.to_radians(),
            node: node.to_radians(),
            peri: peri.to_radians(),
            m0: m0.to_radians(),
            gm,
        }
    }
    pub fn period(self) -> f64 {
        TAU * (self.a.powi(3) / self.gm).sqrt()
    }
    pub fn rotation(self) -> DQuat {
        DQuat::from_rotation_y(self.node)
            * DQuat::from_rotation_x(self.i)
            * DQuat::from_rotation_y(self.peri)
    }
    pub fn normal(self) -> DVec3 {
        self.rotation() * DVec3::Y
    }
    pub fn mean_anomaly(self, time: f64) -> f64 {
        (self.m0 + time.rem_euclid(self.period()) * TAU / self.period()).rem_euclid(TAU)
    }
    pub fn offset(self, time: f64) -> DVec3 {
        self.at_mean(self.mean_anomaly(time))
    }
    pub fn at_mean(self, mean: f64) -> DVec3 {
        let m = (mean + PI).rem_euclid(TAU) - PI;
        let mut eccentric = m;
        for _ in 0..10 {
            let delta =
                (eccentric - self.e * eccentric.sin() - m) / (1.0 - self.e * eccentric.cos());
            eccentric -= delta;
            if delta.abs() < 1e-13 {
                break;
            }
        }
        self.rotation()
            * DVec3::new(
                self.a * (eccentric.cos() - self.e),
                0.,
                -self.a * (1. - self.e * self.e).sqrt() * eccentric.sin(),
            )
    }
}

pub struct System {
    pub elements: [Option<Elements>; MAX_BODIES],
    pub curves: [Vec<DVec3>; MAX_BODIES],
    pub ring_normal: DVec3,
    pub orbit_x: DVec3,
    pub orbit_z: DVec3,
}

impl System {
    pub fn new(eye_height_m: f32) -> Self {
        let mut elements = [None; MAX_BODIES];
        let ossa = Elements::new(BodyId::Vhal, 250_000., 0., 0.5, 0., 0., 180., GM_VHAL);
        elements[BodyId::Ossa.index()] = Some(ossa);
        let radial = ossa.offset(0.).normalize();
        let pole = ossa.normal();
        let up = -radial * LATITUDE.cos() + pole * LATITUDE.sin();
        let east = pole.cross(radial).normalize();
        let south = radial * LATITUDE.sin() + pole * LATITUDE.cos();
        let offset = ossa.offset(0.) + up * (1900. + eye_height_m as f64 / 1000.);
        // 30/5/85 degrees actually gives ~83.2-degree elongation, not the
        // spec table's 87.5. Solve at az=86.5, el=4.6 (both inside the table's
        // tolerances), satisfying the 84--91 elongation AND the prescribed
        // 1.8-degree Vhal / 0.5-degree Ossa orbital inclinations.
        let az = 86.5_f64.to_radians();
        let el = 4.6_f64.to_radians();
        let sun = east * (az.sin() * el.cos()) + up * el.sin() - south * (az.cos() * el.cos());
        // Vhal is at periapsis. Solve distance along the requested solar ray,
        // then the node and argument of periapsis at the specified inclination.
        let radius: f64 = 7_500_000. * (1. - 0.02);
        let b = offset.dot(sun);
        let distance = -b + (b * b + radius * radius - offset.length_squared()).sqrt();
        let position = -offset - sun * distance;
        let inc = 1.8_f64.to_radians();
        let sin_u = position.y / (radius * inc.sin());
        assert!(
            sin_u.abs() <= 1.,
            "opening epoch must fit Vhal's orbital inclination"
        );
        let u = sin_u.asin();
        let unrotated = DQuat::from_rotation_x(inc) * DVec3::new(u.cos(), 0., -u.sin());
        let node = (-position.z).atan2(position.x) - (-unrotated.z).atan2(unrotated.x);
        elements[BodyId::Vhal.index()] = Some(Elements {
            parent: BodyId::Kesh,
            a: 7_500_000.,
            e: 0.02,
            i: inc,
            node,
            peri: u,
            m0: 0.,
            gm: GM_KESH,
        });
        elements[BodyId::Sur.index()] = Some(Elements::new(
            BodyId::Kesh,
            2_600_000.,
            0.08,
            -1.4,
            34.,
            75.,
            214.,
            GM_KESH,
        ));
        elements[BodyId::Nadir.index()] = Some(Elements::new(
            BodyId::Kesh,
            10_500_000.,
            0.05,
            2.6,
            102.,
            28.,
            162.,
            GM_KESH,
        ));
        elements[BodyId::Thren.index()] = Some(Elements::new(
            BodyId::Vhal,
            162_000.,
            0.01,
            2.5,
            38.,
            82.,
            63.,
            GM_VHAL,
        ));
        elements[BodyId::Sable.index()] = Some(Elements::new(
            BodyId::Vhal,
            435_000.,
            0.03,
            -3.,
            117.,
            14.,
            298.,
            GM_VHAL,
        ));
        elements[BodyId::Cinder.index()] = Some(Elements::new(
            BodyId::Nadir,
            60_000.,
            0.01,
            4.,
            19.,
            40.,
            155.,
            GM_NADIR,
        ));
        // A 50-degree spin tilt admits two apparent ring slopes. Solve the
        // branch whose perspective ellipse slopes upward-right, ~24 degrees.
        // The projected ellipse's major axis is NOT the projected tangent at
        // its centre: at this very large angular size those differ by ~14 deg.
        let tangent = pole.cross(radial);
        let los = -offset.normalize();
        let ring_at = |phi: f64| {
            pole * 50_f64.to_radians().cos()
                + (radial * phi.cos() + tangent * phi.sin()) * 50_f64.to_radians().sin()
        };
        let mut lo = 252_f64.to_radians();
        let mut hi = 256_f64.to_radians();
        for _ in 0..48 {
            let mid = (lo + hi) * 0.5;
            if ring_at(mid).dot(los).abs().asin() > 12_f64.to_radians() {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let ring_normal = ring_at((lo + hi) * 0.5).normalize();
        // Director's closer observational revision: 420,000 km, 32 degrees
        // above the sunward face of the ring plane, 75% illuminated. Solve
        // actual phase = (1 + star_direction dot observer_direction)/2.
        // There are two azimuth solutions. Choose the one placing Ossa
        // lower-left and separating Sable from the inner moon arcs. No body
        // or ring positions are changed. Camera up is inertial +Y, no roll.
        let light = (-position).normalize();
        let elevation = 32_f64.to_radians();
        let in_plane = light - ring_normal * light.dot(ring_normal);
        let along = in_plane.normalize();
        let across = ring_normal.cross(along);
        let cos_angle = ((0.5 + light.dot(ring_normal) * elevation.sin())
            / (elevation.cos() * in_plane.length()))
        .clamp(-1., 1.);
        let sin_angle = (1. - cos_angle * cos_angle).sqrt();
        let mut best = f64::INFINITY;
        let mut q = along;
        for sign in [-1., 1.] {
            let horizontal = along * cos_angle + across * (sin_angle * sign);
            let eye = horizontal * elevation.cos() - ring_normal * elevation.sin();
            let forward = -eye;
            let right = forward.cross(DVec3::Y).normalize();
            let camera_up = right.cross(forward);
            let moon = ossa.offset(0.) - eye * 420_000.;
            let depth = moon.dot(forward);
            let mx = moon.dot(right) / depth;
            let my = moon.dot(camera_up) / depth;
            let score = (mx + 0.23).powi(2) + (my + 0.12).powi(2);
            if score < best {
                best = score;
                q = horizontal;
            }
        }
        // The named azimuth is calibrated so the QA sample az=120 is the
        // physical 3/4-lit default. Elevation is measured from the plane.
        let north = DQuat::from_axis_angle(ring_normal, 120_f64.to_radians()) * q;
        let orbit_z = -north;
        let orbit_x = north.cross(ring_normal).normalize();
        let curves = std::array::from_fn(|i| {
            elements[i]
                .map(|e| {
                    (0..=192)
                        .map(|j| e.at_mean(j as f64 * TAU / 192.))
                        .collect()
                })
                .unwrap_or_default()
        });
        Self {
            elements,
            curves,
            ring_normal,
            orbit_x,
            orbit_z,
        }
    }
    pub fn period(&self) -> f64 {
        self.elements[BodyId::Ossa.index()].unwrap().period()
    }
    pub fn positions(&self, t: f64) -> [DVec3; MAX_BODIES] {
        let mut p = [DVec3::ZERO; MAX_BODIES];
        for id in [
            BodyId::Vhal,
            BodyId::Nadir,
            BodyId::Sur,
            BodyId::Ossa,
            BodyId::Thren,
            BodyId::Sable,
            BodyId::Cinder,
        ] {
            let e = self.elements[id.index()].unwrap();
            p[id.index()] = p[e.parent.index()] + e.offset(t);
        }
        p
    }
    pub fn surface_basis(
        &self,
        positions: &[DVec3; MAX_BODIES],
        pos: [f64; 2],
    ) -> (DVec3, DVec3, DVec3) {
        let radial =
            (positions[BodyId::Ossa.index()] - positions[BodyId::Vhal.index()]).normalize();
        let pole = self.elements[BodyId::Ossa.index()].unwrap().normal();
        let e0 = pole.cross(radial).normalize();
        let lat = (LATITUDE - pos[1] / 1_900_000.).clamp(-89_f64.to_radians(), 89_f64.to_radians());
        let lon = pos[0] / (1_900_000. * LATITUDE.cos());
        let equator = -radial * lon.cos() + e0 * lon.sin();
        let up = equator * lat.cos() + pole * lat.sin();
        let east = radial * lon.sin() + e0 * lon.cos();
        let south = -equator * lat.sin() + pole * lat.cos();
        // Expedition grid north (-Z) is the Vhal meridian, i.e. GEODETIC
        // SOUTH at 60 N. This 180-degree compass offset is necessary: a
        // tidally locked giant is equatorward, not poleward. Nothing in the
        // celestial geometry is mirrored; the walking/terrain grid is rotated.
        (east, up, south)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn periods_and_closure() {
        let s = System::new(1.7);
        for id in BodyId::ALL {
            if let Some(e) = s.elements[id.index()] {
                println!(
                    "{} period {:.9} h; elements {:?}",
                    id.name(),
                    e.period() / 3600.,
                    e
                );
                assert!((e.offset(0.) - e.offset(e.period())).length() < 1e-6);
                assert!((e.offset(0.).length() - e.a).abs() <= e.e * e.a + 1e-6);
            }
        }
        assert!((s.period() / 3600. - 20.43153).abs() < 0.00001);
        let n = s.elements[BodyId::Ossa.index()].unwrap().normal();
        assert!((s.ring_normal.dot(n).acos().to_degrees() - 50.).abs() < 1e-8);
    }
}
