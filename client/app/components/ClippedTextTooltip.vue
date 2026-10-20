<template>
  <AppTooltip
    v-if="text !== ''"
    v-model:open="open"
    :text="text"
    :reference="reference ?? undefined"
    :content="{ 'data-bmo-clipped': '' }"
    :ui="{ content: 'pointer-events-none', text: 'text-start' }"
  />
</template>

<script setup lang="ts">
import { clippedAt, clippedWithin, lineAt, type Clipped } from "~/domain/clip";
import { claimPress } from "~/domain/long-press";
import { holdTouchTooltip, releaseTouchTooltip } from "~/domain/tooltips";

// The one layer that shows text cut short, wherever it stands on the screen (docs/face.md, 「The
// screens' shared style」). A screen writes nothing for it: the line under the pointer, under the
// keyboard's focus or under a held finger is found and measured here, as the tooltip is about to
// open, and shown whole. A line that fits carries nothing, and a line that already has a tooltip of
// its own is left to it.

// How long a pointer rests on a line before it is read as reading it. A line the keyboard focused
// is read at once, since the reader has already stopped there.
const HOVER_DELAY_MS = 400;

type Mode = "hover" | "focus" | "touch";

const open = ref(false);
const text = ref("");
const reference = shallowRef<{ getBoundingClientRect: () => DOMRect } | null>(null);

let mode: Mode | null = null;
let hovered: Element | null = null;
let last: { x: number; y: number } | null = null;
let waiting: ReturnType<typeof setTimeout> | null = null;

function stopWaiting(): void {
  if (waiting !== null) {
    clearTimeout(waiting);
    waiting = null;
  }
}

function show(hit: Clipped, how: Mode): void {
  stopWaiting();
  mode = how;
  text.value = hit.text;
  reference.value = {
    getBoundingClientRect: () => new DOMRect(hit.rect.left, hit.rect.top, hit.rect.right - hit.rect.left, hit.rect.bottom - hit.rect.top),
  };
  open.value = true;

  if (how === "touch") {
    holdTouchTooltip(putAway);
  }
}

// What it draws is kept through the close, so the words do not go blank on the way out.
function putAway(): void {
  mode = null;
  stopWaiting();
  open.value = false;
  releaseTouchTooltip(putAway);
}

watch(open, (standing) => {
  if (!standing) {
    releaseTouchTooltip(putAway);
  }
});

// A pointer resting over a line at a time: the line under it is measured as the tooltip opens, so
// the answer follows the width the line has now.
function moved(event: PointerEvent): void {
  if (event.pointerType === "touch") {
    return;
  }

  const element = document.elementFromPoint(event.clientX, event.clientY);

  last = { x: event.clientX, y: event.clientY };

  if (element === hovered) {
    return;
  }

  hovered = element;
  stopWaiting();

  if (mode === "hover") {
    putAway();
  }

  if (element === null) {
    return;
  }

  waiting = setTimeout(() => {
    waiting = null;

    if (hovered !== element || last === null) {
      return;
    }

    const hit = clippedAt(last.x, last.y);

    if (hit !== null) {
      show(hit, "hover");
    }
  }, HOVER_DELAY_MS);
}

// The line the keyboard has stopped on. A control that fits says nothing, and focus a press gave
// — a finger's, or a mouse's — is not the keyboard's and is left alone.
function focused(event: FocusEvent): void {
  if (!(event.target instanceof Element) || !event.target.matches(":focus-visible")) {
    return;
  }

  const hit = clippedWithin(event.target);

  if (hit !== null) {
    show(hit, "focus");
  }
}

function blurred(): void {
  if (mode === "focus") {
    putAway();
  }
}

function pressed(): void {
  if (mode !== "touch") {
    putAway();
  }
}

function scrolled(): void {
  if (mode !== "touch") {
    putAway();
  }
}

function keyed(event: KeyboardEvent): void {
  if (event.key === "Escape" && mode !== null) {
    putAway();
  }
}

let releaseClaim: (() => void) | null = null;

onMounted(() => {
  window.addEventListener("pointermove", moved, true);
  window.addEventListener("pointerdown", pressed, true);
  window.addEventListener("scroll", scrolled, true);
  window.addEventListener("focusin", focused, true);
  window.addEventListener("focusout", blurred, true);
  window.addEventListener("keydown", keyed, true);

  // A held finger reaches a cut line wherever it stands, since the layer cannot know what a tap
  // there does. The line is measured when the press fires, not when the finger landed.
  releaseClaim = claimPress((event) => {
    const line = lineAt(event.clientX, event.clientY);
    const target = event.target instanceof Element ? event.target : null;

    if (line === null || target === null || line.closest("[data-bmo-tooltip]") !== null) {
      return null;
    }

    return {
      target,
      fire: () => {
        const hit = clippedAt(event.clientX, event.clientY);

        if (hit === null) {
          return false;
        }

        show(hit, "touch");

        return true;
      },
    };
  });
});

onBeforeUnmount(() => {
  window.removeEventListener("pointermove", moved, true);
  window.removeEventListener("pointerdown", pressed, true);
  window.removeEventListener("scroll", scrolled, true);
  window.removeEventListener("focusin", focused, true);
  window.removeEventListener("focusout", blurred, true);
  window.removeEventListener("keydown", keyed, true);
  releaseClaim?.();
  stopWaiting();
  releaseTouchTooltip(putAway);
});
</script>
