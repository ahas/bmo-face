<template>
  <g>
    <path :d="part.d" :fill="part.fill" stroke="none" />
    <template v-if="part.inside.length > 0">
      <clipPath :id="clip">
        <path :d="part.d" />
      </clipPath>
      <g :clip-path="`url(#${clip})`">
        <g v-for="(inner, at) in part.inside" :key="at">
          <path :d="inner.d" :fill="inner.fill" stroke="none" />
          <path :d="inner.d" fill="none" :stroke="inner.stroke" :stroke-width="inner.strokeWidth" stroke-linecap="round" stroke-linejoin="round" />
        </g>
      </g>
    </template>
    <path :d="part.d" fill="none" :stroke="part.stroke" :stroke-width="part.strokeWidth" stroke-linecap="round" stroke-linejoin="round" />
  </g>
</template>

<script setup lang="ts">
import type { DrawnPart } from "~/domain/draw";

// One drawn part on the editor's canvas, as the display draws it (docs/face.md, 「The expression
// document」): its fill, what stands inside it cut off at its outline, and its line over the cut.
// `clip` names its clip path, unique on the page.

defineProps<{ part: DrawnPart; clip: string }>();
</script>
