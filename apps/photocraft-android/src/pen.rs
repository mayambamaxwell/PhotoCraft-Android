//! Android tilt/orientation to W3C-style brush angles.
pub fn tilt_degrees(tilt: f32, orientation: f32) -> (f32, f32) {
    if !tilt.is_finite() || !orientation.is_finite() {
        return (0.0, 0.0);
    }
    let tangent = tilt.clamp(0.0, 89_f32.to_radians()).tan();
    ((tangent * orientation.sin()).atan().to_degrees(), (-tangent * orientation.cos()).atan().to_degrees())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tilt_axes_and_invalid_input() {
        let (x, y) = tilt_degrees(45_f32.to_radians(), 0.0);
        assert!(x.abs() < 0.001);
        assert!((y + 45.0).abs() < 0.001);
        let (x, y) = tilt_degrees(45_f32.to_radians(), std::f32::consts::FRAC_PI_2);
        assert!((x - 45.0).abs() < 0.001);
        assert!(y.abs() < 0.001);
        assert_eq!(tilt_degrees(f32::NAN, 0.0), (0.0, 0.0));
    }
}

/// Only suppress fingers; a mouse and a pen remain usable.
#[cfg(test)]
pub fn accept_pointer(is_finger: bool, pen_near: bool, reject_palm: bool) -> bool {
    !is_finger || !pen_near || !reject_palm
}
#[cfg(test)]
mod palm_tests {
    #[test]
    fn suppress_fingers_only_near_pen() {
        assert!(!super::accept_pointer(true, true, true));
        assert!(super::accept_pointer(false, true, true));
        assert!(super::accept_pointer(true, false, true));
        assert!(super::accept_pointer(true, true, false));
    }
}
