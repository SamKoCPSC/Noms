//! One-time model arrivals; scrolling triggers entrances without scrubbing them.
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
pub struct ChapterArrival {
    playback: Playback,
    started: bool,
    chapter: f32,
}
impl ChapterArrival {
    pub fn new(chapter: usize) -> Self {
        Self {
            playback: Playback::default(),
            started: false,
            chapter: chapter as f32,
        }
    }
    pub fn advance(&mut self, progress: f32, seconds: f32) {
        // Direct chapter entry and approaches from either direction share a trigger.
        self.started |= (self.chapter - 0.28..=self.chapter + 0.28).contains(&progress);
        if self.started {
            self.playback.advance(seconds);
        }
    }
    pub fn animating(&self) -> bool {
        self.started && !self.playback.finished()
    }
    pub fn frame(&self) -> f32 {
        // Later rooms use the entire authored sequence, unlike the kitchen intro.
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

pub struct ArrivalPose {
    pub scale: f32,
    pub translation: [f32; 3],
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Room {
    Kitchen,
    Office,
    Potluck,
    Bedroom,
}
impl Room {
    pub fn amount(self, frame: f32, start: f32, end: f32) -> f32 {
        match self {
            Self::Kitchen => amount(frame, start, end),
            Self::Office | Self::Potluck | Self::Bedroom => office_amount(frame, start, end),
        }
    }
}
pub fn arrival_pose(pivot: [f32; 3], authored: [f32; 3], t: f32, room: Room) -> ArrivalPose {
    // glTF Y is up. All rooms have a back wall at -Z; the kitchen/potluck side
    // wall is at -X and the office's window wall is at +X. Enter diagonally
    // through the open front/side, including groups authored as vertical drops.
    let incoming = [
        authored[0].abs().max(0.75) * if room == Room::Office { -1. } else { 1. },
        (authored[1].abs() * 0.25).min(0.45),
        authored[2].abs().max(1.2),
    ];
    let scale = 0.85 + 0.15 * t;
    let mut translation = [0.; 3];
    for axis in 0..3 {
        translation[axis] = pivot[axis] * (1. - scale) + incoming[axis] * (1. - t);
    }
    ArrivalPose { scale, translation }
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
        let mut office = ChapterArrival::new(1);
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
            let mut office = ChapterArrival::new(1);
            office.advance(2., 60.);
            assert_eq!(office.frame(), 1.);
            office.advance(progress, DURATION / 2.);
            assert_eq!(office.frame(), 80.5);
            assert!(office.animating());
        }
    }

    #[test]
    fn philosophy_room_waits_for_its_chapter_and_never_replays() {
        let mut arrival = ChapterArrival::new(3);
        for progress in [0., 1., 2., 2.5] {
            arrival.advance(progress, 10.);
            assert_eq!(arrival.frame(), 1.);
        }
        arrival.advance(2.8, DURATION / 2.);
        assert!(arrival.animating());
        assert_eq!(arrival.frame(), 80.5);
        arrival.advance(1., DURATION);
        for progress in [3., 0., 3.] {
            arrival.advance(progress, 1.);
            assert_eq!(arrival.frame(), 160.);
            assert!(!arrival.animating());
        }
        let mut direct = ChapterArrival::new(3);
        direct.advance(3., DURATION);
        assert_eq!(direct.frame(), 160.);
        assert_eq!(Room::Potluck.amount(1., 1., 25.), 1.);
        assert_eq!(Room::Potluck.amount(1., 115., 146.), 0.);
    }

    #[test]
    fn bedroom_arrives_once_when_creator_chapter_is_approached() {
        let mut arrival = ChapterArrival::new(2);
        arrival.advance(0., 60.);
        assert_eq!(arrival.frame(), 1.);
        arrival.advance(1.8, DURATION / 2.);
        assert!(arrival.animating());
        arrival.advance(2., DURATION);
        for progress in [3., 0., 2.] {
            arrival.advance(progress, 1.);
            assert_eq!(arrival.frame(), 160.);
            assert!(!arrival.animating());
        }
        assert_eq!(Room::Bedroom.amount(1., 1., 25.), 1.);
        assert_eq!(Room::Bedroom.amount(1., 76., 110.), 0.);
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

    #[test]
    fn model_arrivals_stay_on_the_open_sides_of_all_rooms() {
        // Validate the actual exported geometry, including shared pivots for
        // transparent and printed surfaces. Positive uniform scale means its
        // transformed AABB exactly bounds every triangle throughout the path.
        for (bytes, room) in [
            (
                include_bytes!("../../../assets/about/sams-kitchen-detailed.glb").as_slice(),
                Room::Kitchen,
            ),
            (
                include_bytes!("../../../assets/about/sams-office.glb").as_slice(),
                Room::Office,
            ),
            (
                include_bytes!("../../../assets/about/sams-potluck.glb").as_slice(),
                Room::Potluck,
            ),
            (
                include_bytes!("../../../assets/about/sams-bedroom.glb").as_slice(),
                Room::Bedroom,
            ),
        ] {
            let model = super::super::gltf::decode(bytes).unwrap();
            for part in model.parts {
                let mut min = [f32::INFINITY; 3];
                let mut max = [f32::NEG_INFINITY; 3];
                for vertex in
                    model.vertices[part.first * 12..(part.first + part.count) * 12].chunks_exact(12)
                {
                    for axis in 0..3 {
                        min[axis] = min[axis].min(vertex[axis]);
                        max[axis] = max[axis].max(vertex[axis]);
                    }
                }
                for frame in 1..=160 {
                    let t = room.amount(frame as f32, part.start, part.end);
                    let pose = arrival_pose(part.pivot, part.offset, t, room);
                    // Wall-mounted fixtures can already sit in a wall recess at
                    // rest. No moving part may extend farther through either
                    // wall plane than its approved, fully assembled bounds.
                    let side_clearance = if room == Room::Office {
                        max[0] - (max[0] * pose.scale + pose.translation[0])
                    } else {
                        min[0] * pose.scale + pose.translation[0] - min[0]
                    };
                    let back_clearance = min[2] * pose.scale + pose.translation[2] - min[2];
                    assert!(
                        side_clearance >= -0.00001 && back_clearance >= -0.00001,
                        "{} crosses a wall at frame {frame}",
                        part.name
                    );
                    if t < 1. {
                        assert!(side_clearance > 0. && back_clearance > 0.);
                    } else {
                        assert_eq!(pose.scale, 1.);
                        assert_eq!(pose.translation, [0.; 3]);
                    }
                }
            }
        }
    }
}
