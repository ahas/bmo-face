// The loudness the speaking driver reads (docs/face.md, 「The mouth while speaking」): a decoded sound
// read once a frame of the held clock, each frame's loudness in dBFS.

/** The quietest loudness written for a frame: silence. */
export const SILENCE_DB = -120;

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

/** A loudness in dBFS, silence held at the quietest written. */
export function decibels(level: number): number {
  return level > 0 ? Math.max(SILENCE_DB, 20 * Math.log10(level)) : SILENCE_DB;
}

/** A decoded sound's loudness in dBFS, one reading for each frame of the held clock from its first sample. */
export function loudnessByFrame(samples: Float32Array, sampleRate: number, fps: number): number[] {
  const frames = Math.ceil((samples.length / sampleRate) * fps);

  return Array.from({ length: frames }, (_, frame) => {
    const from = Math.round((frame * sampleRate) / fps);
    const to = Math.min(samples.length, Math.round(((frame + 1) * sampleRate) / fps));

    return decibels(rms(samples.subarray(from, to)));
  });
}
