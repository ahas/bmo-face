import { STATES } from "#faces";

import { emotionOf } from "./expression";

import type { Emotion, FaceState } from "#faces";

// Reading BMO's listener stream (docs/face.md, 「What BMO sends, and by which road」): cutting it into
// frames, reading what each says, keeping the cursor, and when to try again.

/** A stream that has said nothing, not even its keep frame, for this long is taken as gone. */
export const SILENCE_MS = 20_000;
/** A briefing heard of later than this after it was sent is not played. */
export const BRIEFING_FRESH_MS = 2 * 60_000;
const FIRST_RETRY_MS = 1_000;
const LAST_RETRY_MS = 60_000;

export interface Frame {
  event: string;
  id: string | null;
  data: string;
}

/** Cuts the stream's text into whole frames, holding a frame not yet whole until the rest comes. */
export class FrameReader {
  private held = "";

  push(text: string): Frame[] {
    this.held += text;

    const frames: Frame[] = [];

    for (;;) {
      const end = /\r?\n\r?\n/.exec(this.held);

      if (end === null) {
        return frames;
      }

      const block = this.held.slice(0, end.index);

      this.held = this.held.slice(end.index + end[0].length);

      const frame = frameOf(block);

      if (frame !== null) {
        frames.push(frame);
      }
    }
  }
}

function frameOf(block: string): Frame | null {
  let event = "message";
  let id: string | null = null;
  const data: string[] = [];

  for (const line of block.split(/\r?\n/)) {
    if (line.startsWith(":")) {
      continue;
    }

    const colon = line.indexOf(":");
    const field = colon === -1 ? line : line.slice(0, colon);
    const value = colon === -1 ? "" : line.slice(colon + 1).replace(/^ /, "");

    if (field === "event") {
      event = value;
    } else if (field === "id") {
      id = value;
    } else if (field === "data") {
      data.push(value);
    }
  }

  return data.length === 0 && id === null && event === "message" ? null : { event, id, data: data.join("\n") };
}

export interface HeardBriefing {
  chatId: number;
  messageId: number;
  text: string;
  // The face it shows: the expression the event carries, else its emotion, else the default face.
  emotion: Emotion;
  sentAt: number;
}

/** What one frame tells the display. */
export type Heard =
  | { kind: "run"; run: string }
  | { kind: "briefing"; briefing: HeardBriefing }
  | { kind: "presence"; state: FaceState; since: number }
  | { kind: "stopped"; chatId: number; messageId: number }
  | { kind: "keep" }
  | { kind: "other" };

function parsed(data: string): Record<string, unknown> | null {
  try {
    const value = JSON.parse(data) as unknown;

    return typeof value === "object" && value !== null ? (value as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}

export function heardOf(frame: Frame): Heard {
  const data = parsed(frame.data);

  if (frame.event === "keep") {
    return { kind: "keep" };
  }

  if (data === null) {
    return { kind: "other" };
  }

  if (frame.event === "run" && typeof data.run === "string") {
    return { kind: "run", run: data.run };
  }

  if (frame.event === "presence" && (STATES as readonly unknown[]).includes(data.state) && typeof data.since === "number") {
    return { kind: "presence", state: data.state as FaceState, since: data.since };
  }

  if (frame.event === "briefing-stopped" && typeof data.chatId === "number" && typeof data.messageId === "number") {
    return { kind: "stopped", chatId: data.chatId, messageId: data.messageId };
  }

  if (frame.event === "bot" && data.type === "briefing" && typeof data.chatId === "number" && typeof data.messageId === "number") {
    const news = data.briefing as Record<string, unknown> | undefined;

    if (news !== undefined && typeof news.sentAt === "number") {
      return {
        kind: "briefing",
        briefing: {
          chatId: data.chatId,
          messageId: data.messageId,
          text: typeof news.text === "string" ? news.text : "",
          // The face the face's own entry answered for the briefing, else the emotion it was spoken with.
          emotion: emotionOf(typeof news.expression === "string" ? news.expression : news.emotion),
          sentAt: news.sentAt,
        },
      };
    }
  }

  return { kind: "other" };
}

export function briefingKey(briefing: { chatId: number; messageId: number }): string {
  return `${briefing.chatId}:${briefing.messageId}`;
}

/** Whether a briefing is still fresh enough to play when it is heard. */
export function isFresh(sentAt: number, now: number): boolean {
  return now - sentAt <= BRIEFING_FRESH_MS;
}

/**
 * The cursor a reconnecting stream resumes from: the last numbered event read, against the run it
 * was read in. A new run of the bot numbers its events afresh, so a cursor from another run is
 * dropped.
 */
export class Cursor {
  after = 0;
  run: string | null = null;

  onRun(run: string): void {
    if (this.run !== null && this.run !== run) {
      this.after = 0;
    }

    this.run = run;
  }

  onId(id: string | null): void {
    const value = Number(id);

    if (id !== null && Number.isSafeInteger(value) && value > this.after) {
      this.after = value;
    }
  }

  query(): string {
    return `?after=${this.after}${this.run === null ? "" : `&run=${encodeURIComponent(this.run)}`}`;
  }
}

/** How long to wait before the next try, doubling from a second up to a minute. */
export function retryDelay(failures: number): number {
  return Math.min(LAST_RETRY_MS, FIRST_RETRY_MS * 2 ** Math.max(0, failures));
}
