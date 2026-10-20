import { describe, expect, test } from "bun:test";

import { emotionOf, LINGER_MS, nextChange, shownFace, SLEEP_AFTER_MS } from "../../app/domain/expression";

import type { FaceMoment } from "../../app/domain/expression";

const NOW = 1_000_000_000;
const quiet: FaceMoment = { speaking: null, spoke: null, presence: { state: "idle", since: NOW - 1_000 }, now: NOW };

// Which face shows (docs/face.md, 「The faces」).
describe("which face shows", () => {
  test("speaking shows the briefing's emotion, with the mouth from the stages", () => {
    expect(shownFace({ ...quiet, speaking: { emotion: "기쁨" }, presence: { state: "writing", since: NOW } })).toEqual({ face: "기쁨", talking: true });
  });

  test("the emotion stays a few seconds after the sound, with the face's own mouth", () => {
    const spoke = { emotion: "슬픔" as const, endedAt: NOW - LINGER_MS + 1 };

    expect(shownFace({ ...quiet, spoke })).toEqual({ face: "슬픔", talking: false });
    expect(shownFace({ ...quiet, spoke: { ...spoke, endedAt: NOW - LINGER_MS } })).toEqual({ face: "idle", talking: false });
  });

  test("otherwise the face is what BMO is doing", () => {
    expect(shownFace({ ...quiet, presence: { state: "thinking", since: NOW } }).face).toBe("thinking");
    expect(shownFace({ ...quiet, presence: { state: "writing", since: NOW - SLEEP_AFTER_MS * 2 } }).face).toBe("writing");
    expect(shownFace(quiet).face).toBe("idle");
  });

  test("a long idle falls asleep, and nothing heard yet reads as idle", () => {
    expect(shownFace({ ...quiet, presence: { state: "idle", since: NOW - SLEEP_AFTER_MS } }).face).toBe("졸림");
    expect(shownFace({ ...quiet, presence: null }).face).toBe("idle");
  });

  test("speaking outranks the linger and the sleep", () => {
    const moment: FaceMoment = { speaking: { emotion: "놀람" }, spoke: { emotion: "화남", endedAt: NOW }, presence: { state: "idle", since: 0 }, now: NOW };

    expect(shownFace(moment).face).toBe("놀람");
  });
});

describe("an emotion a briefing names", () => {
  test("is read as it stands, and none or an unknown one as 기본", () => {
    expect(emotionOf("화남")).toBe("화남");
    expect(emotionOf(undefined)).toBe("기본");
    expect(emotionOf("신남")).toBe("기본");
  });
});

describe("when the face changes next on its own", () => {
  test("at the end of the linger, or when the idle turns to sleep, whichever is first", () => {
    expect(nextChange({ ...quiet, spoke: { emotion: "기쁨", endedAt: NOW } })).toBe(NOW + LINGER_MS);
    expect(nextChange(quiet)).toBe(NOW - 1_000 + SLEEP_AFTER_MS);
    expect(nextChange({ ...quiet, presence: { state: "writing", since: NOW } })).toBeNull();
    expect(nextChange({ ...quiet, speaking: { emotion: "기본" } })).toBeNull();
  });
});
