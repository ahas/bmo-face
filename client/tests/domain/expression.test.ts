import { describe, expect, test } from "bun:test";

import { emotionOf } from "../../app/domain/expression";

// The emotion a briefing names (docs/face.md, 「What BMO sends, and by which road」); which face shows
// from moment to moment is held by the core's own tests.
describe("an emotion a briefing names", () => {
  test("is read as it stands, and none or an unknown one as neutral", () => {
    expect(emotionOf("angry")).toBe("angry");
    expect(emotionOf(undefined)).toBe("neutral");
    expect(emotionOf("신남")).toBe("neutral");
  });
});
