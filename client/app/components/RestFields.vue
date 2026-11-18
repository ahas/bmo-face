<template>
  <section class="flex flex-col gap-4">
    <h2 class="text-base font-semibold">{{ t("face.edit.rest.title") }}</h2>
    <div class="grid grid-cols-2 gap-3">
      <UFormField v-for="stage in STAGES" :key="stage">
        <template #label>
          <AppTooltip :text="t(`face.edit.rest.hint.${stage}`)">
            <span class="cursor-help">{{ t(`face.edit.rest.key.${stage}`) }}</span>
          </AppTooltip>
        </template>
        <NumberInput
          :model-value="restMinutes(document, stage)"
          :step="1"
          :min="stage === 'asleep' ? restMinutes(document, 'blank') : 0"
          :max="REST_MOST_MINUTES"
          :format-options="{ maximumFractionDigits: 1 }"
          :aria-label="t(`face.edit.rest.key.${stage}`)"
          class="w-full"
          @update:model-value="(next: number | null) => next !== null && emit('change', stage, next)"
        />
      </UFormField>
    </div>
  </section>
</template>

<script setup lang="ts">
import { REST_MOST_MINUTES, restMinutes } from "~/domain/edit";

import type { RestStage } from "~/domain/edit";
import type { FaceDocument } from "#faces";

// How long BMO rests before its face changes (docs/face.md, 「Tuning by ear」): the document's two rest
// stages, each a number box in minutes within the core's bounds, named in the language pack with a
// tooltip saying what it changes. Sleep's box starts where the blank face's value stands.

defineProps<{ document: FaceDocument }>();
const emit = defineEmits<{ change: [stage: RestStage, minutes: number] }>();
const { t } = useI18n();

const STAGES: RestStage[] = ["blank", "asleep"];
</script>
