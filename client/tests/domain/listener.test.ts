import { describe, expect, test } from "bun:test";

import { EMOTIONS } from "#faces";

import { BRIEFING_FRESH_MS, Cursor, FrameReader, heardOf, isFresh, retryDelay } from "../../app/domain/listener";

// Reading BMO's listener stream (docs/face.md, 「What BMO sends, and by which road」).
describe("the stream cut into frames", () => {
  test("a frame split across chunks comes out whole once all of it has come, and comments are skipped", () => {
    const reader = new FrameReader();

    expect(reader.push("id: 7\nevent: bot\nda")).toEqual([]);
    expect(reader.push('ta: {"a":1}\n\n: ping\n\nevent: keep\ndata: {}\r\n\r\n')).toEqual([
      { event: "bot", id: "7", data: '{"a":1}' },
      { event: "keep", id: null, data: "{}" },
    ]);
  });
});

describe("what a frame tells the display", () => {
  const frame = (event: string, data: unknown, id: string | null = null) => ({ event, id, data: JSON.stringify(data) });

  test("a briefing, with its emotion, or neutral where it names none", () => {
    const briefing = { type: "briefing", chatId: 42, messageId: 901, briefing: { text: "다 됐어요.", seconds: 2, sentAt: 5, emotion: "happy" } };

    expect(heardOf(frame("bot", briefing, "9"))).toEqual({ kind: "briefing", briefing: { chatId: 42, messageId: 901, text: "다 됐어요.", emotion: "happy", sentAt: 5 } });
    expect(heardOf(frame("bot", { ...briefing, briefing: { text: "", seconds: null, sentAt: 5 } }))).toMatchObject({ briefing: { emotion: "neutral" } });
  });

  test("the expression the face's entry answered, before the emotion, and the default face where it is one this display does not know", () => {
    const [plain, spoken, shown] = EMOTIONS;
    const briefing = { type: "briefing", chatId: 42, messageId: 901, briefing: { text: "done", seconds: 2, sentAt: 5, emotion: spoken, expression: shown } };

    expect(heardOf(frame("bot", briefing))).toMatchObject({ briefing: { emotion: shown } });
    expect(heardOf(frame("bot", { ...briefing, briefing: { ...briefing.briefing, expression: "wink" } }))).toMatchObject({ briefing: { emotion: plain } });
  });

  test("BMO's presence, a stop, the run and the keep frame", () => {
    expect(heardOf(frame("presence", { state: "writing", since: 3 }))).toEqual({ kind: "presence", state: "writing", since: 3 });
    expect(heardOf(frame("briefing-stopped", { chatId: 42, messageId: 901, by: "owner" }))).toEqual({ kind: "stopped", chatId: 42, messageId: 901 });
    expect(heardOf(frame("run", { run: "r1" }))).toEqual({ kind: "run", run: "r1" });
    expect(heardOf(frame("keep", {}))).toEqual({ kind: "keep" });
  });

  test("anything else is nothing to the display", () => {
    expect(heardOf(frame("presence", { state: "dancing", since: 3 }))).toEqual({ kind: "other" });
    expect(heardOf(frame("bot", { type: "todo", chatId: 1 }))).toEqual({ kind: "other" });
    expect(heardOf({ event: "bot", id: null, data: "not json" })).toEqual({ kind: "other" });
  });
});

describe("the cursor", () => {
  test("resumes from the last numbered event of the same run, and starts again on a new run", () => {
    const cursor = new Cursor();

    cursor.onRun("r1");
    cursor.onId("5");
    cursor.onId(null);
    cursor.onId("3");
    expect(cursor.query()).toBe("?after=5&run=r1");

    cursor.onRun("r2");
    expect(cursor.query()).toBe("?after=0&run=r2");
  });
});

describe("a briefing heard late", () => {
  test("is not played past two minutes", () => {
    expect(isFresh(1_000, 1_000 + BRIEFING_FRESH_MS)).toBe(true);
    expect(isFresh(1_000, 1_001 + BRIEFING_FRESH_MS)).toBe(false);
  });
});

describe("trying again", () => {
  test("doubles from a second up to a minute", () => {
    expect([0, 1, 2, 5, 6, 20].map(retryDelay)).toEqual([1_000, 2_000, 4_000, 32_000, 60_000, 60_000]);
  });
});
