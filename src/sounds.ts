/**
 * Three sounds, distinct in kind.
 *
 * **These are placeholders and they are marked as such.** ADR 0002 settled the *semantics* — Hit
 * (loud and unmistakable), Halt (a warning), Resync and Refusal (a quiet blip meaning "that press did
 * not Roll") — and left *which sounds* to
 * [#9](https://github.com/Furizaa/poe-graft/issues/9). What is here is synthesised with WebAudio so
 * the acceptance test has an audible Hit without shipping asset files, which a strict CSP and a
 * bundled installer would both rather we did not.
 *
 * The blip is not polish. A Resync press is physically identical to a Roll press and the human cannot
 * see that the item did not change, so without it they would believe they had rolled.
 */

/**
 * Created on the first user gesture, because a webview will refuse to start an AudioContext without
 * one — and the first gesture of every session is the Arm click.
 */
let context: AudioContext | null = null;

/** Call from a click handler. Safe to call repeatedly. */
export function enableSounds(): void {
  try {
    context ??= new AudioContext();
    if (context.state === "suspended") void context.resume();
  } catch {
    // A machine with no audio device is not a reason for the app to stop working. The Latch already
    // refuses the next press; the sound only tells the human about it sooner.
    context = null;
  }
}

/**
 * Resume a context the webview suspended out from under us.
 *
 * This is a bug fix, not polish. `enableSounds()` used to be called from exactly one place — the Arm
 * button — and a webview is free to suspend an `AudioContext` whenever it likes, including while the
 * game has focus and our window is in the background, which is *every* long Craft Session. Nothing
 * else resumed it, so the Hit sound could go silent mid-craft with no way to get it back short of
 * disarming and re-arming. The Hit sound is the primary signal; the window is only where the human
 * looks afterwards.
 *
 * Called before every sound rather than on a gesture, because by the time a Hit lands there is no
 * gesture to hang it on. A suspended context that was already unlocked once resumes without one.
 */
function wake(): void {
  if (context && context.state === "suspended") void context.resume();
}

type Note = {
  /** Hz. */
  freq: number;
  /** Seconds from now. */
  at: number;
  /** Seconds. */
  length: number;
  /** 0–1. */
  gain: number;
  shape?: OscillatorType;
};

function play(notes: Note[]): void {
  if (!context) return;
  wake();
  const now = context.currentTime;
  for (const note of notes) {
    const oscillator = context.createOscillator();
    const envelope = context.createGain();
    oscillator.type = note.shape ?? "sine";
    oscillator.frequency.value = note.freq;
    // A short ramp at each end, because a square-edged gain change is an audible click on its own.
    envelope.gain.setValueAtTime(0, now + note.at);
    envelope.gain.linearRampToValueAtTime(note.gain, now + note.at + 0.01);
    envelope.gain.linearRampToValueAtTime(0, now + note.at + note.length);
    oscillator.connect(envelope).connect(context.destination);
    oscillator.start(now + note.at);
    oscillator.stop(now + note.at + note.length + 0.02);
  }
}

/** Loud, rising, and longer than the others. The one event this whole app exists for. */
export function playHit(): void {
  play([
    { freq: 880, at: 0, length: 0.12, gain: 0.5 },
    { freq: 1320, at: 0.1, length: 0.14, gain: 0.5 },
    { freq: 1760, at: 0.22, length: 0.4, gain: 0.45 },
  ]);
}

/** A warning: low, square, two pulses. Nothing about it sounds like success. */
export function playHalt(): void {
  play([
    { freq: 196, at: 0, length: 0.16, gain: 0.28, shape: "square" },
    { freq: 147, at: 0.2, length: 0.26, gain: 0.28, shape: "square" },
  ]);
}

/** Quiet and very short — audible over a game, but not something you would call a notification. */
export function playBlip(): void {
  play([{ freq: 1500, at: 0, length: 0.035, gain: 0.09 }]);
}
