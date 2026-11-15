<template>
  <svg
    ref="canvas"
    class="edit-canvas"
    :viewBox="`${-RIM} ${-RIM} ${SCREEN.width + RIM * 2} ${SCREEN.height + RIM * 2}`"
    preserveAspectRatio="xMidYMid meet"
    role="application"
    :aria-label="label"
    @pointermove="drag"
    @pointerup="drop"
    @pointercancel="drop"
  >
    <rect x="0" y="0" :width="SCREEN.width" :height="SCREEN.height" rx="6" :fill="colors.screen" />

    <!-- The rest of the face, for reference only. -->
    <DrawingPaint class="edit-context" aria-hidden="true" :drawing="context" />

    <!-- Editing what stands inside a part: its outline stands dimmed, as the edge they are cut off at. -->
    <g v-if="outline" class="edit-outline" aria-hidden="true">
      <path v-if="outline.fill" v-bind="outline.fill" />
      <clipPath :id="`${id}-outline`">
        <path :d="outline.d" />
      </clipPath>
    </g>

    <g class="edit-parts" :clip-path="outline ? `url(#${id}-outline)` : undefined">
      <g v-for="(part, index) in drawn" :key="index" class="edit-part" :class="{ chosen: index === chosen }" @pointerdown.stop="emit('choose', index, null)">
        <DrawingPaint :drawing="part" />
      </g>
    </g>

    <path v-if="outline?.line" class="edit-outline-line" v-bind="outline.line" aria-hidden="true" />

    <!-- The chosen node's handles, each a thin line from its anchor to its end. -->
    <g v-if="chosenNode !== null && chosen !== null" class="edit-arms" aria-hidden="true">
      <line v-for="arm in arms" :key="arm.kind" :x1="arm.from[0]" :y1="arm.from[1]" :x2="arm.to[0]" :y2="arm.to[1]" />
    </g>

    <g class="edit-handles">
      <template v-for="(part, index) in parts" :key="index">
        <!-- Anchors are drawn last, so where a handle's end stands on its anchor the anchor is taken. -->
        <g
          v-for="handle in ordered(index)"
          :key="`${handle.node}-${handle.kind}`"
          class="edit-handle"
          :class="{ chosen: index === chosen, held: index === chosen && handle.node === chosenNode && handle.kind === 'anchor', end: handle.kind !== 'anchor' }"
        >
          <circle :cx="handle.at[0]" :cy="handle.at[1]" :r="reachOf(handlesFor(index), handlesFor(index).indexOf(handle), fingerReach)" class="edit-handle-reach" @pointerdown.stop="grab($event, index, handle)" />
          <rect v-if="handle.kind !== 'anchor'" :x="handle.at[0] - 1" :y="handle.at[1] - 1" width="2" height="2" class="edit-handle-mark" />
          <circle v-else :cx="handle.at[0]" :cy="handle.at[1]" :r="index === chosen ? 1.4 : 1" class="edit-handle-mark" />
        </g>
      </template>
    </g>
  </svg>
</template>

<script setup lang="ts">
import { SCREEN } from "#faces";

import { partsDrawing } from "#core";

import { paintAttributes, pathData } from "~/domain/draw";
import { handlesOf, moveHandle, reachOf } from "~/domain/edit";

import type { FaceColors, Part, Point } from "#faces";
import type { Handle, HandleAt } from "~/domain/edit";

// The canvas the editor draws on (docs/face.md, 「The editor」): the screen at the display's own
// proportions, the parts being edited in their own paints with a handle on every anchor and on the
// ends of the chosen node's handles, and the rest of the face dimmed behind them. When the parts
// being edited stand inside another, its outline stands dimmed as the edge they are cut off at.

const props = defineProps<{
  colors: FaceColors;
  /** The parts being edited. */
  parts: Part[];
  /** The part they stand inside, when they stand inside one. */
  outer: Part | null;
  /** The rest of the face, drawn dimmed. */
  others: Part[];
  chosen: number | null;
  chosenNode: number | null;
  label: string;
}>();

const emit = defineEmits<{
  change: [parts: Part[]];
  choose: [part: number, node: number | null];
}>();

// Room around the screen, so a handle on its edge can still be taken hold of.
const RIM = 4;

// Clip paths are named in the page's one id space, and the preview beside the canvas names its own.
const id = useId();

const canvas = ref<SVGSVGElement | null>(null);

// A finger's reach — 22px each way, a 44px target — in the screen's units at the size the canvas is
// drawn, never less than the reach a mouse gets.
const FINGER_PX = 22;
const fingerReach = ref(3.5);
let watcher: ResizeObserver | null = null;

function measure(): void {
  const box = canvas.value?.getBoundingClientRect();

  if (box && box.width > 0 && box.height > 0) {
    const unitPx = Math.min(box.width / (SCREEN.width + RIM * 2), box.height / (SCREEN.height + RIM * 2));

    fingerReach.value = Math.max(3.5, FINGER_PX / unitPx);
  }
}

onMounted(() => {
  measure();
  watcher = new ResizeObserver(measure);

  if (canvas.value) {
    watcher.observe(canvas.value);
  }
});
onBeforeUnmount(() => watcher?.disconnect());

// Each part is drawn on its own, so a press on it chooses it.
const drawn = computed(() => props.parts.map((part) => partsDrawing([part], props.colors)));
const context = computed(() => partsDrawing(props.others, props.colors));
const outline = computed(() => {
  if (props.outer === null) {
    return null;
  }

  const { paint } = partsDrawing([{ ...props.outer, inside: [] }], props.colors);
  const noClip = () => "";
  const fill = paint.find((one) => "fill" in one);
  const line = paint.find((one) => "stroke" in one);
  const shape = fill ?? line;

  return shape === undefined
    ? null
    : { d: pathData(shape.path), fill: fill === undefined ? null : paintAttributes(fill, noClip), line: line === undefined ? null : paintAttributes(line, noClip) };
});

const handles = computed(() => props.parts.map((part, index) => handlesOf(part, index === props.chosen ? props.chosenNode : null)));
const handlesFor = (index: number): HandleAt[] => handles.value[index] ?? [];
const ordered = (index: number): HandleAt[] => [...handlesFor(index)].sort((a, b) => Number(a.kind === "anchor") - Number(b.kind === "anchor"));

const arms = computed(() => {
  const node = props.chosen === null || props.chosenNode === null ? undefined : props.parts[props.chosen]?.nodes[props.chosenNode];

  if (node === undefined) {
    return [];
  }

  return (["coming", "leaving"] as const)
    .filter((kind) => node[kind][0] !== 0 || node[kind][1] !== 0)
    .map((kind) => ({ kind, from: node.anchor, to: [node.anchor[0] + node[kind][0], node.anchor[1] + node[kind][1]] as Point }));
});

// What is held, with the part as it stood when it was taken hold of: every step of a drag moves
// that part, so a handle turned on a smooth node keeps its length rather than losing a rounding at
// every step.
const held = ref<{ part: number; from: Part; handle: Handle; pointer: number } | null>(null);

// Where a pointer stands in the screen's own units.
function pointOf(event: PointerEvent): Point | null {
  const matrix = canvas.value?.getScreenCTM();

  if (!matrix) {
    return null;
  }

  const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(matrix.inverse());

  return [point.x, point.y];
}

function grab(event: PointerEvent, part: number, handle: HandleAt): void {
  const from = props.parts[part];

  if (from === undefined) {
    return;
  }

  canvas.value?.setPointerCapture(event.pointerId);
  held.value = { part, from, handle: { node: handle.node, kind: handle.kind }, pointer: event.pointerId };
  emit("choose", part, handle.node);
}

function drag(event: PointerEvent): void {
  if (held.value === null || event.pointerId !== held.value.pointer) {
    return;
  }

  const point = pointOf(event);
  const { part: index, from, handle } = held.value;

  if (point === null || props.parts[index] === undefined) {
    return;
  }

  emit("change", props.parts.map((each, at) => (at === index ? moveHandle(from, handle, point) : each)));
}

function drop(event: PointerEvent): void {
  if (held.value?.pointer === event.pointerId) {
    canvas.value?.releasePointerCapture(event.pointerId);
    held.value = null;
  }
}
</script>

<style scoped>
.edit-canvas {
  display: block;
  width: 100%;
  height: 100%;
  touch-action: none;
  user-select: none;
}

.edit-context,
.edit-outline,
.edit-outline-line {
  opacity: 0.35;
}

.edit-part {
  cursor: pointer;
  transition: opacity var(--bmo-motion-quick) var(--bmo-ease-move);
}

/* Every part is a press that chooses it, so each stands dimmer under the pointer and dimmer again
   while it is pressed, chosen or not. */
.edit-part:not(.chosen) {
  opacity: 0.85;
}

.edit-part:not(.chosen):hover {
  opacity: 0.6;
}

.edit-part:not(.chosen):active {
  opacity: 0.45;
}

.edit-part.chosen:hover {
  opacity: 0.8;
}

.edit-part.chosen:active {
  opacity: 0.65;
}

.edit-arms line {
  stroke: var(--ui-primary);
  stroke-width: 0.3;
}

.edit-handle-reach {
  fill: transparent;
  cursor: grab;
}

.edit-handle-mark {
  fill: #ffffff;
  stroke: var(--ui-primary);
  stroke-width: 0.4;
  pointer-events: none;
}

.edit-handle:not(.chosen) .edit-handle-mark {
  stroke: var(--ui-border-accented);
}

.edit-handle.held .edit-handle-mark,
.edit-handle-reach:hover + .edit-handle-mark {
  fill: var(--ui-primary);
}

.edit-handle-reach:active + .edit-handle-mark {
  fill: var(--ui-color-primary-700);
}

@media (prefers-reduced-motion: reduce) {
  .edit-part {
    transition: none;
  }
}
</style>
