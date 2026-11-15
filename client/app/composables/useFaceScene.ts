import { FaceScene, frameAt, msAt } from "#core";

import type { Drawing } from "#core";
import type { FaceDocument } from "#faces";

// The face from frame to frame, as the core answers it (docs/face.md, 「The held clock」): a scene of the
// document, counted in frames of the held clock from the moment this screen opened, asked for its
// drawing when a frame comes that may change it and at nothing between. What is heard is told to it at
// the frame it is heard; a viewer who asks for less motion is shown every change of face as a cut.

export interface HeldClock {
  /** The frame standing now. */
  frame: () => number;
  /** The moment a frame begins, on the page's own clock. */
  timeOf: (frame: number) => number;
}

export function useFaceScene(document: Ref<FaceDocument | null>, seed: number) {
  const drawing = shallowRef<Drawing | null>(null);
  const face = ref<string | null>(null);
  const origin = performance.now();
  const clock: HeldClock = { frame: () => frameAt(performance.now() - origin), timeOf: (frame) => origin + msAt(frame) };
  let scene: FaceScene | null = null;
  let shown = "";
  let wake: ReturnType<typeof setTimeout> | null = null;

  /** Draws the frame standing now, where it differs from the one shown, and waits for the next change. */
  function draw(): void {
    if (scene === null) {
      return;
    }

    const frame = clock.frame();
    const text = scene.drawingText(frame);

    face.value = scene.faceAt(frame);

    if (text !== shown) {
      shown = text;
      drawing.value = JSON.parse(text) as Drawing;
    }

    if (wake !== null) {
      clearTimeout(wake);
    }

    const next = scene.nextChange(frame);

    wake = next === null ? null : setTimeout(draw, Math.max(0, clock.timeOf(next) - performance.now()) + 1);
  }

  /** Tells the scene something at the frame standing now, and draws what follows. */
  function tell(what: (scene: FaceScene, frame: number) => void): void {
    if (scene !== null) {
      what(scene, clock.frame());
      draw();
    }
  }

  let calm: MediaQueryList | null = null;
  const calmChanged = () => tell((scene) => scene.setCalm(calm?.matches ?? false));

  watch(
    document,
    (current) => {
      if (current === null) {
        return;
      }

      if (scene === null) {
        scene = new FaceScene(current, seed, clock.frame());
        scene.setCalm(calm?.matches ?? false);
      } else {
        scene.setDocument(current);
      }

      draw();
    },
    { immediate: true },
  );

  onMounted(() => {
    calm = globalThis.matchMedia?.("(prefers-reduced-motion: reduce)") ?? null;
    calm?.addEventListener("change", calmChanged);
    calmChanged();
  });
  onBeforeUnmount(() => {
    calm?.removeEventListener("change", calmChanged);

    if (wake !== null) {
      clearTimeout(wake);
    }

    scene?.free();
    scene = null;
  });

  return { drawing, face, clock, tell };
}
