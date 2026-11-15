import { EMOTIONS } from "#faces";

import type { Emotion } from "#faces";

// The emotion a briefing names (docs/face.md, 「What BMO sends, and by which road」). Which face shows
// from moment to moment is the face's core's to decide (「The faces」).

/** The emotion a briefing names, read as the fallback face where it names none or one this display does not know. */
export function emotionOf(word: unknown): Emotion {
  return typeof word === "string" && (EMOTIONS as readonly string[]).includes(word) ? (word as Emotion) : EMOTIONS[0];
}
