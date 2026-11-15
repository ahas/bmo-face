<template>
  <svg
    class="face-view"
    :viewBox="`${-BEZEL} ${-BEZEL} ${SCREEN.width + BEZEL * 2} ${SCREEN.height + BEZEL * 2}`"
    preserveAspectRatio="xMidYMid meet"
    role="img"
    :aria-label="label"
  >
    <rect :x="0" :y="0" :width="SCREEN.width" :height="SCREEN.height" :rx="6" :fill="drawing?.screen ?? screen" />
    <DrawingPaint v-if="drawing" :drawing="drawing" />
  </svg>
</template>

<script setup lang="ts">
import { SCREEN } from "#faces";

import type { Drawing } from "#core";

// BMO's screen with one drawing of the face on it (docs/face.md, 「Showing on any screen」 and 「The
// held clock」): the screen keeps its own proportions and stands as large as the space allows, centred;
// the body around it is the space's own colour. The drawing is the core's, held until the next one
// replaces it; nothing is eased between them.

withDefaults(
  defineProps<{
    drawing: Drawing | null;
    /** The screen's colour while there is no drawing yet. */
    screen?: string;
    label?: string;
  }>(),
  { screen: "transparent", label: "" },
);

// The body's rim around the screen, in the screen's own units.
const BEZEL = 8;
</script>

<style scoped>
.face-view {
  display: block;
  width: 100%;
  height: 100%;
}
</style>
