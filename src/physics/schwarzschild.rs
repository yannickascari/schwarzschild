pub const RS: f32 = 1.0;
pub const R_INNER: f32 = 3.0;
pub const R_OUTER: f32 = 12.0;
pub const R_MAX: f32 = 60.0;

#[inline]
pub fn photon_sphere_radius() -> f32 {
    1.5 * RS
}

#[inline]
pub fn is_captured(r: f32) -> bool {
    r <= RS
}

#[inline]
pub fn orbital_velocity(r: f32) -> f32 {
    (RS / (2.0 * r)).sqrt()
}

#[inline]
pub fn gravitational_redshift(r: f32) -> f32 {
    (1.0 - RS / r).max(0.0).sqrt()
}

#[inline]
pub fn doppler_factor(v_orb: f32, cos_theta: f32) -> f32 {
    1.0 / (1.0 - v_orb * cos_theta).max(0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_photon_sphere_radius() {
        assert_eq!(photon_sphere_radius(), 1.5 * RS);
    }

    #[test]
    fn test_is_captured() {
        assert!(is_captured(0.5));
        assert!(is_captured(RS));
        assert!(!is_captured(RS + 0.001));
        assert!(!is_captured(R_INNER));
    }

    #[test]
    fn test_orbital_velocity() {
        let eps = 1e-6_f32;

        assert!((orbital_velocity(0.5) - 1.0).abs() < eps);
        assert!((orbital_velocity(2.0) - 0.5).abs() < eps);
        assert!(orbital_velocity(3.0) < orbital_velocity(2.0));
    }

    #[test]
    fn test_gravitational_redshift() {
        let eps = 1e-6_f32;

        assert!((gravitational_redshift(RS) - 0.0).abs() < eps);
        assert_eq!(gravitational_redshift(0.5), 0.0);
        assert!((gravitational_redshift(4.0) - 0.75_f32.sqrt()).abs() < eps);
        assert!(gravitational_redshift(100.0) < 1.0);
        assert!(gravitational_redshift(100.0) > gravitational_redshift(10.0));
    }

    #[test]
    fn test_doppler_factor() {
        let v = 0.5_f32;
        let eps = 1e-6_f32;

        assert!((doppler_factor(v, 1.0) - 2.0).abs() < eps);
        assert!((doppler_factor(v, -1.0) - (1.0 / 1.5)).abs() < eps);
        assert!((doppler_factor(v, 0.0) - 1.0).abs() < eps);

        let d_clamped = doppler_factor(1.0, 1.0);
        assert!((d_clamped - 20.0).abs() < eps);
    }

}
