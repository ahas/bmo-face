<template>
  <figure class="flex flex-col gap-1">
    <canvas
      ref="canvas"
      class="spectrum-canvas w-full rounded-md border border-default"
      role="img"
      :aria-label="t('face.edit.preview.strip')"
      @pointermove="hover"
      @pointerleave="hovered = null"
    />
    <figcaption class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted">
      <span v-for="line in LINES" :key="line.key" class="flex items-center gap-1">
        <span class="legend-swatch" :style="{ background: colors[line.key] }" />
        {{ t(`face.edit.preview.${line.key}`) }}
      </span>
      <span v-if="readout" class="ms-auto tabular-nums text-default">{{ readout }}</span>
    </figcaption>
  </figure>
</template>

<script setup lang="ts">
import type { Heard, Pose } from "#core";

// What the preview hears, drawn beneath it (docs/face.md, 「The editor」): the sixteen bands of every
// frame as a heat map, lowest band at the bottom, and under it the level and the openness, roundness
// and teeth the tuning draws from it, each a line across the sound's frames; the frame being shown is
// marked as it plays, and the one under the pointer is read out beside the legend.

const props = defineProps<{ heard: Heard[]; poses: Pose[]; frame: number | null }>();
const { t } = useI18n();

/** The quietest and loudest band the heat map tells apart, in dBFS. */
const QUIET_DB = -90;
const LOUD_DB = -15;
/** The heat map's and the lines' share of the strip's height. */
const HEAT_SHARE = 0.55;
const HEIGHT = 120;

type LineKey = "level" | "open" | "round" | "teeth";

const LINES: { key: LineKey; value: (heard: Heard, pose: Pose | undefined) => number }[] = [
  { key: "level", value: (heard) => (Math.max(QUIET_DB, heard.level) - QUIET_DB) / (0 - QUIET_DB) },
  { key: "open", value: (_, pose) => (pose === undefined ? 0 : pose.open * pose.voice) },
  { key: "round", value: (_, pose) => (pose === undefined ? 0 : pose.round * pose.voice) },
  { key: "teeth", value: (_, pose) => (pose === undefined ? 0 : pose.teeth * pose.voice) },
];

// The heat map is one blue from the surface to its darkest; the lines are the categorical slots in order.
const PALETTE = {
  light: { surface: "#ffffff", heat: [205, 226, 251, 13, 54, 107], level: "#7a7a75", open: "#eb6834", round: "#1baf7a", teeth: "#e87ba4", mark: "#1a1a19", dim: "rgb(255 255 255 / 0.6)" },
  dark: { surface: "#1a1a19", heat: [26, 26, 25, 134, 182, 239], level: "#a3a29a", open: "#d95926", round: "#199e70", teeth: "#d55181", mark: "#f0efec", dim: "rgb(26 26 25 / 0.6)" },
};
const canvas = useTemplateRef<HTMLCanvasElement>("canvas");
// The theme in force where the strip stands, read again whenever the page's classes change.
const dark = ref(false);
const colors = computed(() => PALETTE[dark.value ? "dark" : "light"]);
const hovered = ref<number | null>(null);

const readout = computed(() => {
  const frame = hovered.value ?? props.frame;
  const heard = frame === null ? undefined : props.heard[frame];
  const pose = frame === null ? undefined : props.poses[frame];

  if (frame === null || heard === undefined || pose === undefined) {
    return "";
  }

  const share = (value: number) => (value * pose.voice).toFixed(2);

  return `${frame + 1} · ${t("face.edit.preview.level")} ${Math.round(heard.level)} dB · ${t("face.edit.preview.open")} ${share(pose.open)} · ${t("face.edit.preview.round")} ${share(pose.round)} · ${t("face.edit.preview.teeth")} ${share(pose.teeth)}`;
});

function hover(event: PointerEvent): void {
  const box = canvas.value?.getBoundingClientRect();

  if (box === undefined || props.heard.length === 0) {
    return;
  }

  hovered.value = Math.min(props.heard.length - 1, Math.max(0, Math.floor(((event.clientX - box.left) / box.width) * props.heard.length)));
}

function draw(): void {
  const element = canvas.value;
  const context = element?.getContext("2d");

  if (element === null || element === undefined || context === null || context === undefined) {
    return;
  }

  const scale = globalThis.devicePixelRatio || 1;
  const width = Math.max(1, element.clientWidth);

  element.width = Math.round(width * scale);
  element.height = Math.round(HEIGHT * scale);
  context.setTransform(scale, 0, 0, scale, 0, 0);

  const palette = colors.value;

  context.fillStyle = palette.surface;
  context.fillRect(0, 0, width, HEIGHT);

  const frames = props.heard.length;

  if (frames === 0) {
    return;
  }

  const step = width / frames;
  const heatHeight = HEIGHT * HEAT_SHARE;
  const [r0, g0, b0, r1, g1, b1] = palette.heat as [number, number, number, number, number, number];

  props.heard.forEach((heard, frame) => {
    const bandHeight = heatHeight / heard.bands.length;

    heard.bands.forEach((band, index) => {
      const share = Math.min(1, Math.max(0, (band - QUIET_DB) / (LOUD_DB - QUIET_DB)));
      const mix = (from: number, to: number) => Math.round(from + (to - from) * share);

      context.fillStyle = `rgb(${mix(r0, r1)} ${mix(g0, g1)} ${mix(b0, b1)})`;
      context.fillRect(frame * step, heatHeight - (index + 1) * bandHeight, Math.ceil(step), Math.ceil(bandHeight));
    });
  });

  // The lines' track under the heat map, a 2px gap of surface between them.
  const top = heatHeight + 2;
  const trackHeight = HEIGHT - top - 2;

  for (const line of LINES) {
    context.strokeStyle = palette[line.key];
    context.lineWidth = line.key === "level" ? 1 : 2;
    context.lineJoin = "round";
    context.beginPath();
    props.heard.forEach((heard, frame) => {
      const x = (frame + 0.5) * step;
      const y = top + trackHeight * (1 - line.value(heard, props.poses[frame]));

      if (frame === 0) {
        context.moveTo(x, y);
      } else {
        context.lineTo(x, y);
      }
    });
    context.stroke();
  }

  // While it plays, what is still to come is dimmed, meeting what was shown at a hard edge.
  if (props.frame !== null && props.frame >= 0 && props.frame < frames) {
    context.fillStyle = palette.dim;
    context.fillRect((props.frame + 1) * step, 0, width - (props.frame + 1) * step, HEIGHT);
  }

  for (const [frame, width] of [
    [props.frame, 2],
    [hovered.value, 1],
  ] as const) {
    if (frame !== null && frame >= 0 && frame < frames) {
      context.fillStyle = palette.mark;
      context.fillRect((frame + 0.5) * step - width / 2, 0, width, HEIGHT);
    }
  }
}

watch([() => props.heard, () => props.poses, () => props.frame, hovered, colors], draw, { flush: "post" });

let resized: ResizeObserver | null = null;
let themed: MutationObserver | null = null;

function readTheme(): void {
  dark.value = (canvas.value?.closest(".dark") ?? null) !== null;
}

onMounted(() => {
  readTheme();
  resized = new ResizeObserver(draw);
  themed = new MutationObserver(readTheme);

  if (canvas.value !== null) {
    resized.observe(canvas.value);
  }

  themed.observe(document.documentElement, { attributes: true, attributeFilter: ["class"], subtree: false });
  draw();
});
onBeforeUnmount(() => {
  resized?.disconnect();
  themed?.disconnect();
});
</script>

<style scoped>
.spectrum-canvas {
  height: 120px;
  touch-action: pan-y;
}

.legend-swatch {
  display: inline-block;
  width: 12px;
  height: 2px;
  border-radius: 1px;
}
</style>
