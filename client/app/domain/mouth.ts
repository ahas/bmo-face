// How the sound picks a mouth stage (docs/face.md, 「The mouth while speaking」): the loudness is
// smoothed, mapped from silence to full voice onto the stages, and a stage changes only once the
// loudness is clearly past the boundary.

export interface StagerOptions {
  /** How many mouth stages there are, the first closed and the last widest. */
  stages: number;
  /** The loudness below which the mouth is closed. */
  floor: number;
  /** The loudness at which the mouth is widest. */
  ceiling: number;
  /** How much of the smoothed loudness a rising reading takes at once; the mouth opens quickly. */
  attack: number;
  /** The same for a falling one; the mouth closes more slowly than it opens. */
  release: number;
  /** How far past a boundary, as a share of the whole range, the loudness must go to change stage. */
  margin: number;
}

// Set for BMO's voice as the voice briefings are encoded: speech peaks near 0.3 RMS and a quiet
// pause stays under 0.02.
export const MOUTH_DEFAULTS: Omit<StagerOptions, "stages"> = {
  floor: 0.015,
  ceiling: 0.22,
  attack: 0.6,
  release: 0.25,
  margin: 0.06,
};

/** The loudness of a run of samples: their root mean square. */
export function rms(samples: ArrayLike<number>): number {
  if (samples.length === 0) {
    return 0;
  }

  let sum = 0;

  for (let index = 0; index < samples.length; index++) {
    const sample = samples[index]!;

    sum += sample * sample;
  }

  return Math.sqrt(sum / samples.length);
}

export class MouthStager {
  private level = 0;
  private stage = 0;
  private readonly options: StagerOptions;

  constructor(stages: number, options: Partial<Omit<StagerOptions, "stages">> = {}) {
    this.options = { ...MOUTH_DEFAULTS, ...options, stages: Math.max(1, stages) };
  }

  /** Takes one loudness reading and answers the stage the mouth stands at now. */
  push(loudness: number): number {
    const { floor, ceiling, attack, release, margin, stages } = this.options;
    const target = Math.min(1, Math.max(0, (loudness - floor) / (ceiling - floor)));

    this.level += (target - this.level) * (target > this.level ? attack : release);

    // The bands divide the range evenly. Rising, the level must stand a margin inside a higher band
    // to reach it; falling, a margin below the band it is leaving. Between the two it stays put.
    const band = (level: number) => Math.min(stages - 1, Math.max(0, Math.floor(level * stages)));
    const up = band(this.level - margin);
    const down = band(this.level + margin);

    if (up > this.stage) {
      this.stage = up;
    } else if (down < this.stage) {
      this.stage = down;
    }

    return this.stage;
  }

  reset(): void {
    this.level = 0;
    this.stage = 0;
  }
}

/**
 * The loudness of a decoded sound, one reading per `stepMs`, each over a window of `windowMs` —
 * what drives the mouth when the display plays nothing aloud.
 */
export function envelope(samples: Float32Array, sampleRate: number, stepMs = 1000 / 60, windowMs = 40): number[] {
  const step = Math.max(1, Math.round((sampleRate * stepMs) / 1000));
  const window = Math.max(1, Math.round((sampleRate * windowMs) / 1000));
  const readings: number[] = [];

  for (let start = 0; start < samples.length; start += step) {
    readings.push(rms(samples.subarray(start, Math.min(samples.length, start + window))));
  }

  return readings;
}
