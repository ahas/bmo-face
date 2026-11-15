import { FACTS } from "#core";

import { isFresh } from "~/domain/listener";
import { loudnessByFrame } from "~/domain/mouth";

import type { FaceScene } from "#core";
import type { HeldClock } from "~/composables/useFaceScene";
import type { HeardBriefing } from "~/domain/listener";

// Plays a briefing and hands the face's core what it needs to move the mouth with it (docs/face.md,
// 「The mouth while speaking」). The sound is fetched with the screen's token and decoded whole into the
// page's own audio graph before it plays, its loudness read once a frame of the held clock, and the
// core plans every drawing over it at once. The sound is started on the audio clock so that it is heard
// the core's lead after the first drawing shows, the output latency the browser reports taken off; with
// `?sound=off` nothing plays and the mouth runs on the frame clock alone.

export interface SceneTeller {
  clock: HeldClock;
  tell: (what: (scene: FaceScene, frame: number) => void) => void;
}

/** How many frames the first drawing waits after a sound is decoded, so its own frame is not already past. */
const SETTLE_FRAMES = 1;

export function useSpeaker(options: { token: Ref<string | null>; silent: boolean; face: SceneTeller }) {
  const speaking = ref<HeardBriefing | null>(null);
  /** The browser holds sound back until the screen is touched once. */
  const suspended = ref(false);

  let context: AudioContext | null = null;
  let source: AudioBufferSourceNode | null = null;
  let ending: ReturnType<typeof setTimeout> | null = null;
  let waiting: HeardBriefing | null = null;
  // Each briefing's play is numbered, so a stop or a newer briefing ends the older one.
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

    if (ending !== null) {
      clearTimeout(ending);
      ending = null;
    }

    try {
      source?.stop();
    } catch {
      // It had already ended.
    }

    source = null;
    speaking.value = null;
    options.face.tell((scene, frame) => scene.stop(frame));
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

    const loudness = loudnessByFrame(buffer.getChannelData(0), buffer.sampleRate, FACTS.fps);
    const { clock } = options.face;
    // The first drawing shows a frame from now; the sound is heard the lead after it.
    const heard = clock.frame() + SETTLE_FRAMES + FACTS.lead;
    const heardIn = (clock.timeOf(heard) - performance.now()) / 1000;

    speaking.value = briefing;
    options.face.tell((scene) => scene.speak({ loudness, seed: briefing.messageId }, briefing.emotion, heard));

    if (options.silent) {
      ending = setTimeout(() => finish(which), heardIn * 1000 + buffer.duration * 1000);

      return;
    }

    // What the browser says its output path delays the sound by, taken off its start.
    const latency = context.outputLatency || context.baseLatency || 0;

    source = context.createBufferSource();
    source.buffer = buffer;
    source.connect(context.destination);
    source.onended = () => finish(which);
    source.start(context.currentTime + Math.max(0, heardIn - latency));
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

  return { speaking, suspended, play, stop, resume };
}
