<template>
  <section class="flex flex-col gap-4">
    <h2 class="text-base font-semibold">{{ t("face.edit.tuning.title") }}</h2>
    <div v-for="group in groups" :key="group.name" class="flex flex-col gap-3">
      <h3 class="text-sm font-medium text-muted">{{ t(`face.edit.tuning.group.${group.name}`) }}</h3>
      <div class="grid grid-cols-2 gap-3">
        <UFormField v-for="value in group.values" :key="value.key" :class="value.kind === 'span' ? 'col-span-2' : ''">
          <template #label>
            <AppTooltip :text="t(`face.edit.tuning.hint.${value.key}`)">
              <span class="cursor-help">{{ t(`face.edit.tuning.key.${value.key}`) }}</span>
            </AppTooltip>
          </template>
          <div v-if="value.kind === 'span'" class="grid grid-cols-2 gap-2">
            <NumberInput
              v-for="end in ENDS"
              :key="end"
              :model-value="spanOf(value.key)[end]"
              :step="stepOf(value)"
              :min="value.least"
              :max="value.most"
              :aria-label="`${t(`face.edit.tuning.key.${value.key}`)} ${t(end === 0 ? 'face.edit.tuning.from' : 'face.edit.tuning.to')}`"
              class="w-full"
              @update:model-value="(next: number | null) => setSpan(value.key, end, next)"
            />
          </div>
          <NumberInput
            v-else
            :model-value="numberOf(value.key)"
            :step="stepOf(value)"
            :min="value.least"
            :max="value.most"
            :format-options="{ maximumFractionDigits: 2 }"
            :aria-label="t(`face.edit.tuning.key.${value.key}`)"
            class="w-full"
            @update:model-value="(next: number | null) => next !== null && emit('change', value.key, next)"
          />
        </UFormField>
        <div v-if="group.name === 'time'" class="col-span-2">
          <AppTooltip :text="t(`face.edit.tuning.hint.${FACTS.tuning.switch}`)">
            <USwitch
              :model-value="tuning[FACTS.tuning.switch] === true"
              :label="t(`face.edit.tuning.key.${FACTS.tuning.switch}`)"
              @update:model-value="(on) => emit('change', FACTS.tuning.switch, on)"
            />
          </AppTooltip>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { FACTS } from "#core";

import type { Tuning } from "#faces";

// The talking set's tuning (docs/face.md, 「Tuning by ear」): every value the core declares, each a number
// box within its bounds — a span of two hertz two boxes — named in the language pack, with a tooltip
// saying what it moves, in the four groups the editor offers them in. The switch for drawing on twos
// stands in the time group.

const props = defineProps<{ tuning: Tuning }>();
const emit = defineEmits<{ change: [key: string, value: Tuning[string]] }>();
const { t } = useI18n();

const ENDS = [0, 1] as const;

/** The keys each group offers, in order; a key the core declares that no group names stands in the mouth group. */
const GROUPS: { name: string; keys: string[] }[] = [
  { name: "sound", keys: ["floor", "range", "lowHz", "midHz", "highHz"] },
  { name: "mouth", keys: ["open", "jaw", "jawHz", "round", "roundHz", "teeth", "tongue", "corners", "cues"] },
  { name: "time", keys: ["attack", "release", "glide", "rest"] },
  { name: "eyes", keys: ["eyePulse", "eyeOnset", "eyeSquint", "eyeShine"] },
];

interface Declared {
  key: string;
  kind: "number" | "span";
  least: number;
  most: number;
}

const declared: Declared[] = [
  ...FACTS.tuning.numbers.map(({ key, least, most }) => ({ key, kind: "number" as const, least, most })),
  ...FACTS.tuning.spans.map(({ key, least, most }) => ({ key, kind: "span" as const, least, most })),
];
const grouped = new Set(GROUPS.flatMap((group) => group.keys));
const groups = GROUPS.map((group) => ({
  name: group.name,
  values: [
    ...group.keys.flatMap((key) => declared.filter((one) => one.key === key)),
    ...(group.name === "mouth" ? declared.filter((one) => !grouped.has(one.key)) : []),
  ],
}));

/** A step that moves a value by about a hundredth of its bounds, on a round figure. */
function stepOf(value: Declared): number {
  const span = value.most - value.least;

  if (span >= 1000) return 10;
  if (span >= 100) return 5;
  if (span >= 10) return 1;
  if (span >= 4) return 0.1;

  return 0.05;
}

function numberOf(key: string): number {
  const value = props.tuning[key];

  return typeof value === "number" ? value : 0;
}

function spanOf(key: string): [number, number] {
  const value = props.tuning[key];

  return Array.isArray(value) ? value : [0, 0];
}

function setSpan(key: string, end: 0 | 1, next: number | null): void {
  if (next !== null) {
    const span: [number, number] = [...spanOf(key)];

    span[end] = next;
    emit("change", key, span);
  }
}
</script>
