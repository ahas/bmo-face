import { describe, expect, test } from "bun:test";

import { envelope, MouthStager, rms } from "../../app/domain/mouth";

// How the sound picks a mouth stage (docs/face.md, 「The mouth while speaking」).
describe("loudness", () => {
  test("is the root mean square of the samples, and silence is nought", () => {
    expect(rms([])).toBe(0);
    expect(rms([0, 0, 0])).toBe(0);
    expect(rms([0.5, -0.5, 0.5, -0.5])).toBeCloseTo(0.5);
  });

  test("a decoded sound is read once per step over a short window", () => {
    const samples = new Float32Array(48_000);

    samples.fill(0.3, 24_000);

    const readings = envelope(samples, 48_000, 100, 40);

    expect(readings).toHaveLength(10);
    expect(readings[0]).toBe(0);
    expect(readings[9]).toBeCloseTo(0.3);
  });
});

describe("the mouth stage", () => {
  // Fed long enough for the smoothing to settle.
  function settle(stager: MouthStager, loudness: number, times = 40): number {
    let stage = 0;

    for (let i = 0; i < times; i++) {
      stage = stager.push(loudness);
    }

    return stage;
  }

  test("silence closes the mouth and full voice opens it widest", () => {
    const stager = new MouthStager(5);

    expect(settle(stager, 0)).toBe(0);
    expect(settle(stager, 1)).toBe(4);
    expect(settle(stager, 0)).toBe(0);
  });

  test("a voice in between opens it partway", () => {
    const stager = new MouthStager(5, { floor: 0, ceiling: 1 });

    expect(settle(stager, 0.5)).toBe(2);
  });

  test("a single spike does not flash the mouth open", () => {
    const stager = new MouthStager(5, { floor: 0, ceiling: 1 });

    settle(stager, 0);
    expect(stager.push(1)).toBeLessThan(4);
    expect(settle(stager, 0)).toBe(0);
  });

  test("a steady vowel on a boundary does not flutter between two stages", () => {
    const stager = new MouthStager(5, { floor: 0, ceiling: 1, attack: 1, release: 1, margin: 0.05 });
    const seen = new Set<number>();

    // Hovering around the 2|3 boundary at 0.6, within the margin.
    for (const loudness of [0.62, 0.58, 0.61, 0.59, 0.62, 0.58, 0.6]) {
      seen.add(stager.push(loudness));
    }

    expect(seen.size).toBe(1);
  });

  test("the mouth opens faster than it closes", () => {
    const opening = new MouthStager(5, { floor: 0, ceiling: 1 });
    const closing = new MouthStager(5, { floor: 0, ceiling: 1 });
    let toOpen = 0;
    let toClose = 0;

    while (opening.push(1) < 4) toOpen++;
    settle(closing, 1);
    while (closing.push(0) > 0) toClose++;

    expect(toOpen).toBeLessThan(toClose);
  });
});
