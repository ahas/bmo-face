import { Cursor, FrameReader, heardOf, retryDelay, SILENCE_MS } from "~/domain/listener";

import type { Heard } from "~/domain/listener";

// Holds BMO's listener stream open for as long as the display stands: read with a streaming fetch,
// since the token rides in a header, resumed from its cursor, taken as gone after twenty seconds of
// silence and opened again with a growing wait (docs/face.md, 「What BMO sends, and by which road」).

export interface ListenerHandlers {
  heard: (heard: Heard) => void;
  /** BMO refused the token: the screen is no longer signed in. */
  refused: () => void;
}

export function useListener(token: Ref<string | null>, handlers: ListenerHandlers) {
  const connected = ref(false);
  const cursor = new Cursor();
  let aborter: AbortController | null = null;
  let retry: ReturnType<typeof setTimeout> | null = null;
  let failures = 0;
  let stopped = false;

  async function open(): Promise<void> {
    if (stopped || token.value === null) {
      return;
    }

    const current = new AbortController();
    let silence: ReturnType<typeof setTimeout> | null = null;
    const heardSomething = () => {
      if (silence !== null) {
        clearTimeout(silence);
      }

      silence = setTimeout(() => current.abort(), SILENCE_MS);
    };

    aborter = current;
    heardSomething();

    try {
      const response = await fetch(bmoUrl(`/api/briefing/events${cursor.query()}`), {
        headers: { authorization: `Bearer ${token.value}`, accept: "text/event-stream" },
        signal: current.signal,
        cache: "no-store",
      });

      if (response.status === 401 || response.status === 403) {
        handlers.refused();

        return;
      }

      if (!response.ok || response.body === null) {
        throw new Error(String(response.status));
      }

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      const frames = new FrameReader();

      for (;;) {
        const { done, value } = await reader.read();

        if (done) {
          break;
        }

        heardSomething();

        for (const frame of frames.push(decoder.decode(value, { stream: true }))) {
          cursor.onId(frame.id);

          const heard = heardOf(frame);

          if (heard.kind === "run") {
            cursor.onRun(heard.run);
            connected.value = true;
            failures = 0;
          }

          handlers.heard(heard);
        }
      }
    } catch {
      // Dropped, refused by a BMO that is not there, or taken as gone after its silence; the same
      // wait and try again answers all three.
    } finally {
      if (silence !== null) {
        clearTimeout(silence);
      }
    }

    connected.value = false;

    if (!stopped && token.value !== null) {
      retry = setTimeout(() => void open(), retryDelay(failures++));
    }
  }

  function close(): void {
    stopped = true;
    aborter?.abort();

    if (retry !== null) {
      clearTimeout(retry);
    }
  }

  onMounted(() => {
    void open();
  });
  onBeforeUnmount(close);

  // A screen signed in again starts listening at once.
  watch(token, (now, before) => {
    if (now !== null && before === null) {
      stopped = false;
      failures = 0;
      void open();
    }
  });

  return { connected };
}
