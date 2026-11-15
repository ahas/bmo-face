<template>
  <section v-if="target.tab === 'faces' && pair !== null" class="flex flex-col gap-4">
    <USwitch :model-value="pair.right === null" :label="t('face.edit.fields.mirror')" @update:model-value="(on) => emit('mirror', on)" />
    <UFormField v-if="reach !== null" :label="t('face.edit.fields.reach')">
      <div class="grid grid-cols-2 gap-2">
        <NumberInput
          v-for="axis in AXES"
          :key="axis"
          :model-value="reach[axis]"
          :step="0.2"
          :min="0"
          :max="LIMITS.maxReach"
          :aria-label="`${t('face.edit.fields.reach')} ${t(axis === 0 ? 'face.edit.fields.x' : 'face.edit.fields.y')}`"
          class="w-full"
          @update:model-value="(value: number | null) => setReach(axis, value)"
        />
      </div>
    </UFormField>
  </section>

  <UFormField v-else-if="talkMouth !== null" :label="t('face.edit.fields.size')">
    <USelect :model-value="talkMouth.size" :items="sizeItems" class="w-full" @update:model-value="(value) => emit('size', value as Size)" />
  </UFormField>
</template>

<script setup lang="ts">
import { faceFor, LIMITS, SIZES } from "#faces";

import { isPair } from "~/domain/edit";

import type { FaceDocument, Pair, Point, Size } from "#faces";
import type { EditTarget } from "~/domain/edit";

// What the editor sets of the element being edited as a whole (docs/face.md, 「The editor」): whether a
// pair's right side mirrors its left, how far the eyes move with the gaze, and a talking mouth's size.

const props = defineProps<{ document: FaceDocument; target: EditTarget }>();
const emit = defineEmits<{ mirror: [mirrored: boolean]; reach: [reach: Point]; size: [size: Size] }>();
const { t } = useI18n();

const AXES = [0, 1] as const;

const pair = computed<Pair | null>(() => {
  const { target } = props;

  return target.tab === "faces" && isPair(target.element) ? faceFor(props.document, target.face)[target.element] : null;
});
const reach = computed<Point | null>(() => {
  const { target } = props;

  return target.tab === "faces" && target.element === "eyes" ? faceFor(props.document, target.face).eyes.reach : null;
});
const talkMouth = computed(() => (props.target.tab === "talk" ? (props.document.talk[props.target.family]?.mouths[props.target.mouth] ?? null) : null));
const sizeItems = computed(() => SIZES.map((size) => ({ label: t(`face.edit.size.${size}`), value: size })));

function setReach(axis: 0 | 1, value: number | null): void {
  if (reach.value !== null && value !== null) {
    const next: Point = [...reach.value];

    next[axis] = value;
    emit("reach", next);
  }
}
</script>
