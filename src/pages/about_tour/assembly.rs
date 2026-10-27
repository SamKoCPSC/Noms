//! One-time model arrivals, with scroll used only to trigger the office entrance.
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
#[derive(Default)]
pub struct OfficeArrival {
    playback: Playback,
    started: bool,
}
impl OfficeArrival {
    pub fn advance(&mut self, progress: f32, seconds: f32) {
        // The office becomes visible as the camera approaches the second stop.
        // Direct #why entry and approaching from either direction use the same trigger.
        self.started |= (0.72..=1.28).contains(&progress);
        if self.started {
            self.playback.advance(seconds);
        }
    }
    pub fn animating(&self) -> bool {
        self.started && !self.playback.finished()
    }
    pub fn frame(&self) -> f32 {
        // Office timing spans the entire authored sequence, unlike the kitchen intro.
        1. + 159. * (self.playback.elapsed / DURATION)
    }
}
pub fn amount(frame: f32, start: f32, end: f32) -> f32 {
    // Keep the architectural shell intact before appliances begin to arrive.
    group_amount(frame, start, end, 60.)
}
pub fn office_amount(frame: f32, start: f32, end: f32) -> f32 {
    group_amount(frame, start, end, 25.)
}
fn group_amount(frame: f32, start: f32, end: f32, foundation_end: f32) -> f32 {
    if end <= foundation_end {
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

    #[test]
    fn office_waits_for_approach_then_plays_independently_of_scroll() {
        let mut office = OfficeArrival::default();
        office.advance(0., 60.);
        assert_eq!(office.frame(), 1.);
        assert!(!office.animating());
        office.advance(0.8, 0.);
        assert!(office.animating());
        office.advance(0.8, DURATION / 2.);
        assert_eq!(office.frame(), 80.5);
        // Leaving the stop never reverses or resets an entrance already in progress.
        office.advance(3., DURATION);
        assert_eq!(office.frame(), 160.);
        assert!(!office.animating());
        for progress in [0., 1., 2., 1.] {
            office.advance(progress, 1.);
            assert_eq!(office.frame(), 160.);
            assert!(!office.animating());
        }
    }

    #[test]
    fn office_direct_entry_and_reverse_approach_start_the_entrance() {
        for progress in [1., 1.2] {
            let mut office = OfficeArrival::default();
            office.advance(2., 60.);
            assert_eq!(office.frame(), 1.);
            office.advance(progress, DURATION / 2.);
            assert_eq!(office.frame(), 80.5);
            assert!(office.animating());
        }
    }

    #[test]
    fn office_shell_is_present_while_furniture_and_accessories_arrive() {
        assert_eq!(office_amount(1., 1., 25.), 1.);
        for (start, end) in [(18., 48.), (32., 65.), (72., 104.), (116., 145.)] {
            assert_eq!(office_amount(1., start, end), 0.);
            assert_eq!(office_amount((start + end) / 2., start, end), 0.5);
            assert_eq!(office_amount(160., start, end), 1.);
        }
    }
}
