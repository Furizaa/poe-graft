/**
 * PROTOTYPE — throwaway. See `src/prototype/README.md`.
 *
 * Three candidate sound sets × three signals, auditionable side by side.
 *
 * ADR 0002 settled the *semantics* and #9 owes an answer on the *sounds*: Hit (loud and
 * unmistakable), Halt (a warning that sounds like nothing good), and a quiet blip meaning "that press
 * did not Roll". The blip is not polish — a Resync press is physically identical to a Roll press and
 * the human cannot see that the item did not change.
 *
 * Two constraints that decide this, and that a quiet room hides:
 *
 * 1. **The Hit has to be recognisable over a running game.** Path of Exile's combat audio is loud and
 *    broadband in the low-mid range. Anything that competes there loses; anything clean, tonal and
 *    rhythmically regular cuts through, because games almost never produce those.
 * 2. **The blip must not be mistakable for the Hit** — and the current placeholders get this wrong.
 *    `playBlip()` is 1500 Hz and `playHit()` runs 880 → 1320 → **1760** Hz, so the blip sits *inside*
 *    the Hit's spectral range and is distinguished only by being shorter and quieter. Over game audio
 *    that is the least reliable difference available. Every candidate set below separates them by
 *    **pitch direction** instead: the Hit goes high and up, the blip stays low and flat.
 *
 * Audition these with the game running, not in a quiet room. That is the whole test.
 */

export type Note = {
  /** Hz. */
  freq: number;
  /** Seconds from now. */
  at: number;
  /** Seconds. */
  length: number;
  /** 0–1. */
  gain: number;
  shape?: OscillatorType;
  /** Exponential decay rather than a linear ramp down — what makes a struck bell sound struck. */
  pluck?: boolean;
};

export type SoundSet = {
  key: string;
  name: string;
  /** Why this set might win. Shown in the bench. */
  argument: string;
  hit: Note[];
  halt: Note[];
  blip: Note[];
};

/**
 * The three candidates.
 *
 * They disagree about what "unmistakable" means: a musical resolution, a physical object being
 * struck, or an alarm that refuses to be pleasant.
 */
export const soundSets: SoundSet[] = [
  {
    key: "chime",
    name: "Chime — the current placeholder",
    argument:
      "A rising sine arpeggio. Pleasant, unambiguous as success, and already proven audible on device. Its weakness is that sine tones are the first thing a busy mix swallows, and its blip is the one that collides with the Hit.",
    hit: [
      { freq: 880, at: 0, length: 0.12, gain: 0.5 },
      { freq: 1320, at: 0.1, length: 0.14, gain: 0.5 },
      { freq: 1760, at: 0.22, length: 0.4, gain: 0.45 },
    ],
    halt: [
      { freq: 196, at: 0, length: 0.16, gain: 0.28, shape: "square" },
      { freq: 147, at: 0.2, length: 0.26, gain: 0.28, shape: "square" },
    ],
    // Kept as-is so the collision described above is audible rather than described.
    blip: [{ freq: 1500, at: 0, length: 0.035, gain: 0.09 }],
  },
  {
    key: "bell",
    name: "Bell — struck, with a long tail",
    argument:
      "Two strikes with inharmonic partials and an exponential decay. A struck object reads as an event rather than a notification, and the long tail survives a moment of loud combat: if you miss the strike you still hear the ring. The blip is a low soft thud — the opposite end of the spectrum from the Hit.",
    hit: [
      { freq: 1174, at: 0, length: 1.1, gain: 0.42, pluck: true },
      { freq: 1760, at: 0, length: 0.9, gain: 0.22, pluck: true },
      { freq: 2637, at: 0.005, length: 0.5, gain: 0.11, pluck: true },
      { freq: 1568, at: 0.16, length: 1.3, gain: 0.38, pluck: true },
      { freq: 2349, at: 0.16, length: 1.0, gain: 0.18, pluck: true },
    ],
    halt: [
      { freq: 110, at: 0, length: 0.7, gain: 0.34, shape: "triangle", pluck: true },
      { freq: 165, at: 0, length: 0.6, gain: 0.16, shape: "triangle", pluck: true },
      { freq: 98, at: 0.26, length: 0.9, gain: 0.32, shape: "triangle", pluck: true },
    ],
    blip: [{ freq: 320, at: 0, length: 0.045, gain: 0.11, shape: "triangle", pluck: true }],
  },
  {
    key: "alarm",
    name: "Alarm — refuses to be pleasant",
    argument:
      "A fast alternating two-tone, the pattern a phone ring or an emergency tone uses, because nothing in a game soundtrack is that regular. Hardest to miss and hardest to like — and the Hit is the one event this whole app exists for, which may be the trade worth making.",
    hit: [
      { freq: 1568, at: 0, length: 0.09, gain: 0.5, shape: "square" },
      { freq: 2093, at: 0.1, length: 0.09, gain: 0.5, shape: "square" },
      { freq: 1568, at: 0.2, length: 0.09, gain: 0.5, shape: "square" },
      { freq: 2093, at: 0.3, length: 0.09, gain: 0.5, shape: "square" },
      { freq: 1568, at: 0.4, length: 0.09, gain: 0.5, shape: "square" },
      { freq: 2093, at: 0.5, length: 0.26, gain: 0.5, shape: "square" },
    ],
    halt: [
      { freq: 233, at: 0, length: 0.34, gain: 0.3, shape: "sawtooth" },
      { freq: 220, at: 0.03, length: 0.34, gain: 0.24, shape: "sawtooth" },
      { freq: 175, at: 0.4, length: 0.5, gain: 0.3, shape: "sawtooth" },
      { freq: 165, at: 0.43, length: 0.5, gain: 0.24, shape: "sawtooth" },
    ],
    blip: [{ freq: 260, at: 0, length: 0.03, gain: 0.1, shape: "square" }],
  },
];

let context: AudioContext | null = null;

/** Bench-local, so auditioning cannot interfere with the real `src/sounds.ts`. */
export function benchPlay(notes: Note[]): void {
  try {
    context ??= new AudioContext();
    if (context.state === "suspended") void context.resume();
  } catch {
    return;
  }
  const now = context.currentTime;
  for (const note of notes) {
    const oscillator = context.createOscillator();
    const envelope = context.createGain();
    oscillator.type = note.shape ?? "sine";
    oscillator.frequency.value = note.freq;
    const start = now + note.at;
    envelope.gain.setValueAtTime(0, start);
    envelope.gain.linearRampToValueAtTime(note.gain, start + 0.008);
    if (note.pluck) {
      // exponentialRamp cannot reach 0, so decay to near-silence and cut.
      envelope.gain.exponentialRampToValueAtTime(0.0001, start + note.length);
    } else {
      envelope.gain.linearRampToValueAtTime(0, start + note.length);
    }
    oscillator.connect(envelope).connect(context.destination);
    oscillator.start(start);
    oscillator.stop(start + note.length + 0.02);
  }
}
