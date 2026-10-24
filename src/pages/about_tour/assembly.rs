//! Scroll positions are shared by the camera, chapter panels and kitchen assembly.
//! The opening gets one extra segment to assemble before forward camera travel.
pub fn camera_progress(travel: f32) -> f32 {
    (travel - 1.).clamp(0., 3.)
}
pub fn frame(travel: f32) -> f32 {
    40. + 120. * (travel / 0.85).clamp(0., 1.)
}
pub fn amount(frame: f32, start: f32, end: f32) -> f32 {
    // Keep the architectural shell intact before appliances begin to arrive.
    if end <= 60. {
        return 1.;
    }
    let t = ((frame - start) / (end - start)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kitchen_finishes_before_camera_leaves_the_opening() {
        assert_eq!(camera_progress(0.85), 0.);
        assert_eq!(frame(0.85), 160.);
        for (travel, chapter) in [(0., 0.), (1., 0.), (2., 1.), (3., 2.), (4., 3.)] {
            assert_eq!(camera_progress(travel), chapter);
        }
    }
    #[test]
    fn foundations_are_complete_at_the_start() {
        for (start, end) in [(1., 18.), (12., 40.), (30., 60.)] {
            assert_eq!(amount(40., start, end), 1.);
        }
    }
    #[test]
    fn assembly_is_clamped_and_reversible_without_time_or_history() {
        assert_eq!(amount(-20., 112., 142.), 0.);
        assert_eq!(amount(160., 112., 142.), 1.);
        let positions: Vec<_> = (0..100)
            .map(|i| amount(frame(i as f32 / 100.), 112., 142.))
            .collect();
        assert!(positions.windows(2).all(|p| p[0] <= p[1]));
        for (i, original) in positions.iter().enumerate().rev() {
            assert_eq!(amount(frame(i as f32 / 100.), 112., 142.), *original);
        }
    }
}
