//! The kitchen assembles once after loading, independently of camera travel.
const DURATION: f32 = 3.;
#[derive(Default)]
pub struct Playback {
    elapsed: f32,
}
impl Playback {
    pub fn advance(&mut self, seconds: f32) {
        self.elapsed = (self.elapsed + seconds.max(0.)).min(DURATION);
    }
    pub fn finish(&mut self) {
        self.elapsed = DURATION;
    }
    pub fn finished(&self) -> bool {
        self.elapsed >= DURATION
    }
    pub fn frame(&self) -> f32 {
        40. + 120. * (self.elapsed / DURATION)
    }
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
    fn intro_advances_without_scrolling_and_stays_complete() {
        let mut playback = Playback::default();
        assert_eq!(playback.frame(), 40.);
        assert!(!playback.finished());
        playback.advance(DURATION / 2.);
        assert_eq!(playback.frame(), 100.);
        playback.advance(DURATION);
        assert_eq!(playback.frame(), 160.);
        assert!(playback.finished());
        playback.advance(-DURATION);
        playback.advance(60.);
        assert_eq!(playback.frame(), 160.);
    }
    #[test]
    fn direct_chapter_entry_can_skip_an_unseen_intro() {
        let mut playback = Playback::default();
        playback.finish();
        assert!(playback.finished());
        assert_eq!(playback.frame(), 160.);
    }
    #[test]
    fn foundations_are_complete_at_the_start() {
        for (start, end) in [(1., 18.), (12., 40.), (30., 60.)] {
            assert_eq!(amount(40., start, end), 1.);
        }
    }
    #[test]
    fn groups_ease_into_place_during_the_intro() {
        assert_eq!(amount(-20., 112., 142.), 0.);
        assert_eq!(amount(160., 112., 142.), 1.);
        let mut playback = Playback::default();
        let mut previous = 0.;
        for _ in 0..100 {
            playback.advance(DURATION / 90.);
            let position = amount(playback.frame(), 112., 142.);
            assert!(position >= previous);
            previous = position;
        }
        assert_eq!(previous, 1.);
    }
}
