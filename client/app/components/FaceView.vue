<template>
  <svg
    class="face-view"
    :viewBox="`${-BEZEL} ${-BEZEL} ${SCREEN.width + BEZEL * 2} ${SCREEN.height + BEZEL * 2}`"
    preserveAspectRatio="xMidYMid meet"
    role="img"
    :aria-label="label"
  >
    <rect :x="0" :y="0" :width="SCREEN.width" :height="SCREEN.height" :rx="6" :fill="colors.screen" />
    <g v-for="group in groups" :key="group.name" :class="['face-group', `face-${group.name}`, { still: group.still }]">
      <g v-for="(part, index) in group.parts" :key="index">
        <path :style="fillOf(part)" />
        <template v-if="part.inside.length > 0">
          <clipPath :id="clipOf(group.name, index)">
            <path :style="{ d: shape(part) }" />
          </clipPath>
          <g :clip-path="`url(#${clipOf(group.name, index)})`">
            <g v-for="(inner, at) in part.inside" :key="at">
              <path :style="fillOf(inner)" />
              <path :style="lineOf(inner)" stroke-linecap="round" stroke-linejoin="round" />
            </g>
          </g>
        </template>
        <path :style="lineOf(part)" stroke-linecap="round" stroke-linejoin="round" />
      </g>
    </g>
  </svg>
</template>

<script setup lang="ts">
import { PART_GROUPS, SCREEN } from "#faces";

import { drawPart } from "~/domain/draw";

import type { Face, FaceColors, Part, PartGroup } from "#faces";
import type { DrawnPart } from "~/domain/draw";

// BMO's screen with one face on it (docs/face.md, 「Showing on any screen」 and 「The faces」): the
// screen keeps its own proportions and stands as large as the space allows, centred; the body around
// it is the space's own colour. A part is drawn as its fill, then what stands inside it cut off at
// its outline, then its line over the cut edge. Parts are keyed by their place in their group, and
// every path holds the same run of commands, so a face turning into another moves each part rather
// than cutting to it.

const props = withDefaults(
  defineProps<{
    colors: FaceColors;
    face: Face;
    /** The mouth drawn in place of the face's own, while speaking. */
    mouth?: Part[] | null;
    label?: string;
  }>(),
  { mouth: null, label: "" },
);

// The body's rim around the screen, in the screen's own units.
const BEZEL = 8;

// A clip path is named in the page's one id space, and a page may hold more than one face.
const id = useId();
const clipOf = (group: PartGroup, index: number) => `${id}-${group}-${index}`;

const shape = (part: DrawnPart) => `path('${part.d}')`;
const fillOf = (part: DrawnPart) => ({ d: shape(part), fill: part.fill, stroke: "none" });
const lineOf = (part: DrawnPart) => ({ d: shape(part), fill: "none", stroke: part.stroke, strokeWidth: part.strokeWidth });

const groups = computed(() =>
  PART_GROUPS.map((name) => {
    const talking = name === "mouth" && props.mouth !== null;
    const parts = talking ? (props.mouth as Part[]) : props.face[name];

    return {
      name,
      // A mouth moving with the sound changes every frame and is never eased.
      still: talking,
      parts: parts.map((part) => drawPart(part, props.colors)),
    };
  }),
);
</script>

<style scoped>
.face-view {
  display: block;
  width: 100%;
  height: 100%;
}

.face-group path {
  transition:
    d 180ms ease-out,
    fill 180ms ease-out,
    stroke 180ms ease-out,
    stroke-width 180ms ease-out;
}

.face-group.still path {
  transition: none;
}

@media (prefers-reduced-motion: reduce) {
  .face-group path {
    transition: none;
  }
}
</style>
