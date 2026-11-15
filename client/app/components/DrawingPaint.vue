<template>
  <g>
    <defs>
      <clipPath v-for="(clip, index) in drawing.clips" :id="clipId(index)" :key="index">
        <path :d="pathData(clip)" />
      </clipPath>
    </defs>
    <path v-for="(paint, index) in drawing.paint" :key="index" v-bind="paintAttributes(paint, clipId)" />
  </g>
</template>

<script setup lang="ts">
import { paintAttributes, pathData } from "~/domain/draw";

import type { Drawing } from "#core";

// The paint of one drawing the core answered, laid down in order, each cut off at the clip it names
// (docs/face.md, 「The face's core」). Clip paths are named in the page's one id space, and a page may
// hold more than one drawing.

defineProps<{ drawing: Drawing }>();

const id = useId();
const clipId = (index: number) => `${id}-clip-${index}`;
</script>
