import { isFresh } from "~/domain/listener";
import { envelope, MouthStager, rms } from "~/domain/mouth";

import type { Emotion } from "#faces";
import type { HeardBriefing } from "~/domain/listener";

// Plays a briefing and moves the mouth with it (docs/face.md, 「The mouth while speaking」). The sound
// is fetched with the screen's token and decoded into the page's own audio graph. Heard, the mouth
// follows an analyser on what is playing; with `?sound=off`, nothing plays and the mouth follows the
// decoded sound's loudness on a clock.

const STEP_MS = 1000 / 60;

export function useSpeaker(options: { token: Ref<string | null>; silent: boolean; stages: Ref<number> }) {
  const speaking = ref<HeardBriefing | null>(null);
  const spoke = ref<{ emotion: Emotion; endedAt: number } | null>(null);
  const stage = ref(0);
  /** The browser holds sound back until the screen is touched once. */
  const suspended = ref(false);

  let context: AudioContext | null = null;
  let source: AudioBufferSourceNode | null = null;
  let frame: number | null = null;
  let waiting: HeardBriefing | null = null;
  // Each briefing's play is numbered, so a stop or a newer briefing ends the older one's loop.
  let playing = 0;

  function audio(): AudioContext {
    context ??= new AudioContext();
    suspended.value = !options.silent && context.state === "suspended";

    return context;
  }

  function finish(which: number): void {
    if (which !== playing || speaking.value === null) {
      return;
    }

    if (frame !== null) {
      cancelAnimationFrame(frame);
      frame = null;
    }

    try {
      source?.stop();
    } catch {
      // It had already ended.
    }

    source = null;
    spoke.value = { emotion: speaking.value.emotion, endedAt: Date.now() };
    speaking.value = null;
    stage.value = 0;
  }

  async function play(briefing: HeardBriefing): Promise<void> {
    const context = audio();

    if (suspended.value) {
      // Held until the first touch, and played then only if it is still fresh.
      waiting = briefing;

      return;
    }

    // A newer briefing takes over from one still playing.
    stop();

    const which = ++playing;
    const response = await fetch(bmoUrl(`/api/media/audio/${briefing.chatId}/${briefing.messageId}`), {
      headers: { authorization: `Bearer ${options.token.value ?? ""}` },
    }).catch(() => null);

    if (response === null || !response.ok || which !== playing) {
      return;
    }

    const buffer = await context.decodeAudioData(await response.arrayBuffer()).catch(() => null);

    if (buffer === null || which !== playing) {
      return;
    }

    const stager = new MouthStager(options.stages.value);

    speaking.value = briefing;
    stage.value = 0;

    if (options.silent) {
      const readings = envelope(buffer.getChannelData(0), buffer.sampleRate, STEP_MS);
      const began = performance.now();
      const step = () => {
        if (which !== playing) {
          return;
        }

        const index = Math.floor((performance.now() - began) / STEP_MS);

        if (index >= readings.length) {
          finish(which);

          return;
        }

        stage.value = stager.push(readings[index]!);
        frame = requestAnimationFrame(step);
      };

      frame = requestAnimationFrame(step);

      return;
    }

    const analyser = context.createAnalyser();
    const samples = new Float32Array(1024);

    analyser.fftSize = samples.length;
    source = context.createBufferSource();
    source.buffer = buffer;
    source.connect(analyser);
    analyser.connect(context.destination);
    source.onended = () => finish(which);
    source.start();

    const step = () => {
      if (which !== playing) {
        return;
      }

      analyser.getFloatTimeDomainData(samples);
      stage.value = stager.push(rms(samples));
      frame = requestAnimationFrame(step);
    };

    frame = requestAnimationFrame(step);
  }

  /** Stops what is playing; the same as the sound running out. */
  function stop(): void {
    finish(playing);
  }

  /** The first touch: lets the sound out, and plays what was held if it is still fresh. */
  async function resume(): Promise<void> {
    const context = audio();

    await context.resume().catch(() => undefined);
    suspended.value = !options.silent && context.state === "suspended";

    const held = waiting;

    waiting = null;

    if (!suspended.value && held !== null && isFresh(held.sentAt, Date.now())) {
      await play(held);
    }
  }

  onMounted(() => {
    audio();
  });
  onBeforeUnmount(() => {
    stop();
    void context?.close();
  });

  return { speaking, spoke, stage, suspended, play, stop, resume };
}
