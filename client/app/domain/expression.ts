import { EMOTIONS } from "#faces";

import type { Emotion, FaceName, FaceState } from "#faces";

// Which face shows (docs/face.md, 「The faces」). First match wins: speaking with the briefing's
// emotion, then that emotion for a few seconds after it, then asleep after a long idle, otherwise the
// face of what BMO is doing.

/** How long the emotion stays after the sound ends. */
export const LINGER_MS = 4_000;
/** How long BMO is idle before it falls asleep. */
export const SLEEP_AFTER_MS = 10 * 60_000;

export interface FaceMoment {
  /** The briefing playing now, with the emotion it was spoken with; null when nothing plays. */
  speaking: { emotion: Emotion } | null;
  /** The last briefing that ended, and when. */
  spoke: { emotion: Emotion; endedAt: number } | null;
  /** What BMO is doing, and since when; null before the stream has said. */
  presence: { state: FaceState; since: number } | null;
  now: number;
}

export interface ShownFace {
  face: FaceName;
  /** Whether the mouth comes from the mouth stages rather than the face's own. */
  talking: boolean;
}

/** The emotion a briefing names, read as `기본` where it names none or one this display does not know. */
export function emotionOf(word: unknown): Emotion {
  return typeof word === "string" && (EMOTIONS as readonly string[]).includes(word) ? (word as Emotion) : "기본";
}

export function shownFace(moment: FaceMoment): ShownFace {
  if (moment.speaking !== null) {
    return { face: moment.speaking.emotion, talking: true };
  }

  if (moment.spoke !== null && moment.now - moment.spoke.endedAt < LINGER_MS) {
    return { face: moment.spoke.emotion, talking: false };
  }

  const presence = moment.presence ?? { state: "idle", since: moment.now };

  if (presence.state === "idle" && moment.now - presence.since >= SLEEP_AFTER_MS) {
    return { face: "졸림", talking: false };
  }

  return { face: presence.state, talking: false };
}

/** When the shown face will next change with nothing new heard, so the display looks again then. */
export function nextChange(moment: FaceMoment): number | null {
  if (moment.speaking !== null) {
    return null;
  }

  const times: number[] = [];

  if (moment.spoke !== null && moment.now - moment.spoke.endedAt < LINGER_MS) {
    times.push(moment.spoke.endedAt + LINGER_MS);
  }

  if (moment.presence?.state === "idle" && moment.now - moment.presence.since < SLEEP_AFTER_MS) {
    times.push(moment.presence.since + SLEEP_AFTER_MS);
  }

  return times.length === 0 ? null : Math.min(...times);
}
