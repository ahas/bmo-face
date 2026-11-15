import { describe, expect, test } from "bun:test";

import { decibels, loudnessByFrame, rms, SILENCE_DB } from "../../app/domain/mouth";

// The loudness the speaking driver reads (docs/face.md, 「The mouth while speaking」).
describe("loudness", () => {
  test("is the root mean square of the samples, and silence is nought", () => {
    expect(rms([])).toBe(0);
    expect(rms([0, 0, 0])).toBe(0);
    expect(rms([0.5, -0.5, 0.5, -0.5])).toBeCloseTo(0.5);
  });

  test("is read in dBFS, silence at the quietest written", () => {
    expect(decibels(1)).toBe(0);
    expect(decibels(0.1)).toBeCloseTo(-20);
    expect(decibels(0)).toBe(SILENCE_DB);
  });

  test("of a decoded sound is read once a frame of the held clock, over that frame's samples", () => {
    const samples = new Float32Array(48_000);

    samples.fill(0.5, 24_000);

    const frames = loudnessByFrame(samples, 48_000, 24_000 / 1_001);

    expect(frames).toHaveLength(24);
    expect(frames[0]).toBe(SILENCE_DB);
    expect(frames[23]).toBeCloseTo(decibels(0.5));
  });
});
