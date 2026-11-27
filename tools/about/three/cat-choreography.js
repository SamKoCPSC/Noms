// Cue times refer to the supplied recording. The excerpt runs from 0:30 to 1:00.
import { MathUtils } from 'three';

const beatSeconds = 60 / 150;
const tau = 2 * Math.PI;
const smooth = MathUtils.smootherstep;

// First kick of the drop in the trimmed recording.
export function recordingBeat(elapsed) {
  return (elapsed - 3.535) / beatSeconds;
}

// Finish on the original heading, hold for half a beat, then reverse on the next burst.
export function burstSpin(beat, turns) {
  const within = MathUtils.euclideanModulo(beat, 2);
  if (within >= 1.5) return 0;
  const progress = within / 1.5;
  const direction = Math.floor(beat / 2) % 2 ? -1 : 1;
  return direction * turns * (tau * progress - 0.25 * Math.sin(tau * progress));
}

export function danceToRecording(cat, shadow, home, restYaw, elapsed, length) {
  const time = 30 + elapsed;
  const beat = recordingBeat(elapsed);
  const phase = MathUtils.euclideanModulo(beat, 1);
  const bounce = Math.sin(Math.PI * phase) ** 2;
  const landing = Number.isFinite(length) ? smooth(length - elapsed, 0, 4) : 1;
  const motion = smooth(elapsed, 0, 0.4) * landing;
  const lift = smooth(beat, 16, 24) * landing;
  const finale = smooth(beat, 40, 48);
  const drop = smooth(time, 33.135, 33.535);
  const turns = beat < 0 ? 2 : beat < 16 ? 3 : beat < 40 ? 4 : 5;
  const spin = beat >= -8 && beat < 64 ? burstSpin(beat, turns) : 0;
  const glide = ((beat - 24) * tau) / 16;
  const roll = beat >= 40 ? tau * smooth(MathUtils.euclideanModulo(beat, 16), 10, 13) : 0;
  cat.position.set(
    home.x + lift * (1.45 + (1.25 + 0.65 * finale) * Math.sin(glide)),
    home.y +
      motion * bounce * (0.09 + 0.16 * drop) +
      lift * (2.05 + 0.52 * Math.cos(glide * 2) + finale * 0.8),
    home.z + lift * (1.15 + finale * 0.6) * Math.sin(2 * glide)
  );
  cat.rotation.set(
    motion * (0.16 * Math.sin(tau * phase) + lift * 0.3 * Math.cos(glide)),
    restYaw + spin,
    motion * (0.12 * Math.sin(Math.PI * beat) + lift * 0.4 * Math.sin(glide)) + roll
  );
  const squash = motion * (0.09 + 0.07 * drop) * Math.cos(tau * phase);
  cat.scale.set(1 + squash * 0.55, 1 - squash, 1 + squash * 0.55);
  shadow.material.opacity = (1 - lift) * (1 - 0.35 * motion * bounce);
}
