<template>
  <section v-if="target.tab === 'faces' && (pair !== null || shown?.element === 'mouth')" class="flex flex-col gap-4">
    <USwitch v-if="pair !== null" :model-value="pair.right === null" :label="t('face.edit.fields.mirror')" @update:model-value="(on) => emit('mirror', on)" />
    <template v-if="eyes !== null">
      <UFormField :label="t('face.edit.fields.reach')">
        <div class="grid grid-cols-2 gap-2">
          <NumberInput
            v-for="axis in AXES"
            :key="axis"
            :model-value="eyes.reach[axis]"
            :step="0.2"
            :min="0"
            :max="LIMITS.maxReach"
            :aria-label="`${t('face.edit.fields.reach')} ${t(axis === 0 ? 'face.edit.fields.x' : 'face.edit.fields.y')}`"
            class="w-full"
            @update:model-value="(value: number | null) => setReach(axis, value)"
          />
        </div>
      </UFormField>
      <UFormField :label="t('face.edit.fields.voice')">
        <div class="grid grid-cols-3 gap-2">
          <UFormField v-for="key in EYE_VOICE" :key="key" :label="t(`face.edit.fields.${key}`)" :ui="{ label: 'text-sm text-muted' }">
            <NumberInput
              :model-value="eyes.voice[key]"
              :step="0.01"
              :min="0"
              :max="LIMITS.maxEyeVoice"
              :format-options="{ maximumFractionDigits: 2 }"
              :aria-label="`${t('face.edit.fields.voice')} ${t(`face.edit.fields.${key}`)}`"
              class="w-full"
              @update:model-value="(value: number | null) => setVoice(key, value)"
            />
          </UFormField>
        </div>
      </UFormField>
    </template>
    <UFormField v-if="shown?.element === 'mouth'" :label="t('face.edit.fields.corners')">
      <NumberInput
        :model-value="shownFace?.corners ?? 0"
        :step="0.1"
        :min="-1"
        :max="1"
        :format-options="{ maximumFractionDigits: 2 }"
        class="w-full"
        @update:model-value="(value: number | null) => value !== null && emit('corners', value)"
      />
    </UFormField>
  </section>

  <section v-else-if="anchor !== null" class="flex flex-col gap-4">
    <UFormField :label="t('face.edit.fields.place')">
      <div class="grid grid-cols-2 gap-2">
        <UFormField v-for="axis in AXES" :key="axis" :label="t(axis === 0 ? 'face.edit.fields.open' : 'face.edit.fields.wide')" :ui="{ label: 'text-sm text-muted' }">
          <NumberInput
            :model-value="anchor.at[axis]"
            :step="0.01"
            :min="0"
            :max="1"
            :format-options="{ maximumFractionDigits: 3 }"
            :aria-label="`${t('face.edit.fields.place')} ${t(axis === 0 ? 'face.edit.fields.open' : 'face.edit.fields.wide')}`"
            class="w-full"
            @update:model-value="(value: number | null) => setPlace(axis, value)"
          />
        </UFormField>
      </div>
    </UFormField>
    <div class="flex items-center justify-between gap-2">
      <USwitch :model-value="anchor.teeth" :label="t('face.edit.fields.teeth')" @update:model-value="(on) => emit('teeth', on)" />
      <UButton size="sm" icon="i-material-symbols-straighten-outline" color="neutral" variant="outline" :label="t('face.edit.action.measure')" @click="emit('measure')" />
    </div>
  </section>
</template>

<script setup lang="ts">
import { EYE_VOICE, faceFor, LIMITS } from "#faces";

import { isPair } from "~/domain/edit";

import type { Anchor, Eyes, EyeVoice, EyeVoiceKey, Face, FaceDocument, Pair, Point } from "#faces";
import type { EditTarget } from "~/domain/edit";

// What the editor sets of the element being edited as a whole (docs/face.md, 「The editor」): whether a
// pair's right side mirrors its left, how far the eyes move with the gaze and answer the voice, how far
// the face's talking mouth lifts its corners, and where an anchor of the talking set stands — set by
// hand or placed where its drawing measures — and whether its teeth stand together.

const props = defineProps<{ document: FaceDocument; target: EditTarget }>();
const emit = defineEmits<{
  mirror: [mirrored: boolean];
  reach: [reach: Point];
  voice: [voice: EyeVoice];
  corners: [corners: number];
  place: [at: Point];
  teeth: [teeth: boolean];
  measure: [];
}>();
const { t } = useI18n();

const AXES = [0, 1] as const;

const shown = computed(() => (props.target.tab === "faces" ? props.target : null));
const shownFace = computed<Face | null>(() => (shown.value === null ? null : faceFor(props.document, shown.value.face)));
const pair = computed<Pair | null>(() => (shown.value !== null && isPair(shown.value.element) ? shownFace.value![shown.value.element] : null));
const eyes = computed<Eyes | null>(() => (shown.value?.element === "eyes" ? shownFace.value!.eyes : null));
const anchor = computed<Anchor | null>(() => (props.target.tab === "talk" ? (props.document.talk.anchors[props.target.anchor] ?? null) : null));

function setReach(axis: 0 | 1, value: number | null): void {
  if (eyes.value !== null && value !== null) {
    const next: Point = [...eyes.value.reach];

    next[axis] = value;
    emit("reach", next);
  }
}

function setVoice(key: EyeVoiceKey, value: number | null): void {
  if (eyes.value !== null && value !== null) {
    emit("voice", { ...eyes.value.voice, [key]: value });
  }
}

function setPlace(axis: 0 | 1, value: number | null): void {
  if (anchor.value !== null && value !== null) {
    const next: Point = [...anchor.value.at];

    next[axis] = value;
    emit("place", next);
  }
}
</script>
