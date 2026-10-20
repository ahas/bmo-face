<template>
  <div class="flex flex-col gap-4">
    <div v-if="inside" class="flex">
      <UButton icon="i-material-symbols-arrow-back" color="neutral" variant="ghost" :label="t('edit.action.outline')" @click="emit('leave')" />
    </div>

    <p v-if="part === null" class="text-sm text-muted">{{ empty ? t("edit.fields.empty") : t("edit.fields.none") }}</p>

    <template v-else>
      <section class="flex flex-col gap-4">
        <USwitch :model-value="part.closed" :label="t('edit.fields.closed')" :disabled="part.closed && holdsInside" @update:model-value="setOpenness" />
        <div class="grid grid-cols-2 gap-4">
          <UFormField :label="t('edit.fields.fill')">
            <USelect :model-value="part.fill ?? NONE" :items="fillItems" :disabled="!part.closed" class="w-full" @update:model-value="setFill" />
          </UFormField>
          <UFormField :label="t('edit.fields.line')">
            <USelect :model-value="part.line?.paint ?? NONE" :items="lineItems" class="w-full" @update:model-value="setLine" />
          </UFormField>
        </div>
        <UFormField v-if="part.line" :label="t('edit.fields.width')">
          <NumberInput :model-value="part.line.width" :step="0.2" :min="LIMITS.minWidth" :max="LIMITS.maxWidth" class="w-full" @update:model-value="setWidth" />
        </UFormField>
      </section>

      <USeparator />

      <section class="flex flex-col gap-4">
        <div class="flex items-center gap-2">
          <USelect :model-value="chosenNode ?? undefined" :items="nodeItems" :placeholder="t('edit.fields.node')" :aria-label="t('edit.fields.node')" class="min-w-0 flex-1" @update:model-value="(value) => emit('chooseNode', Number(value))" />
          <AppTooltip :text="t('edit.action.add_node')">
            <UButton icon="i-material-symbols-add" color="neutral" variant="ghost" :aria-label="t('edit.action.add_node')" :disabled="part.nodes.length >= LIMITS.maxNodes" @click="moreNode" />
          </AppTooltip>
          <AppTooltip :text="t('edit.action.remove_node')">
            <UButton
              icon="i-material-symbols-remove"
              color="neutral"
              variant="ghost"
              :aria-label="t('edit.action.remove_node')"
              :disabled="node === null || part.nodes.length <= LIMITS.minNodes"
              @click="dropNode"
            />
          </AppTooltip>
        </div>

        <p v-if="node === null" class="text-sm text-muted">{{ t("edit.fields.node_none") }}</p>
        <template v-else>
          <UFormField v-for="row in rows" :key="row.kind" :label="t(`edit.fields.${row.kind}`)">
            <div class="grid grid-cols-2 gap-2">
              <NumberInput
                v-for="axis in AXES"
                :key="axis"
                :model-value="row.value[axis]"
                :step="0.5"
                :aria-label="`${t(`edit.fields.${row.kind}`)} ${t(axis === 0 ? 'edit.fields.x' : 'edit.fields.y')}`"
                class="w-full"
                @update:model-value="(value: number | null) => setNumber(row.kind, axis, value)"
              />
            </div>
          </UFormField>
          <USwitch :model-value="node.kind === 'smooth'" :label="t('edit.fields.smooth')" @update:model-value="(value) => emitPart(setSmooth(part!, chosenNode!, value))" />
        </template>
      </section>

      <template v-if="!inside && part.closed">
        <USeparator />

        <section class="flex flex-col gap-2">
          <div class="flex items-center justify-between gap-2">
            <h3 class="text-base font-semibold">{{ t("edit.fields.inside") }}</h3>
            <div class="flex items-center gap-2">
              <UButton icon="i-material-symbols-add" color="neutral" variant="outline" :label="t('edit.action.add_teeth')" :disabled="insideFull" @click="emitPart(addInside(part!, 'teeth'))" />
              <UButton icon="i-material-symbols-add" color="neutral" variant="outline" :label="t('edit.action.add_tongue')" :disabled="insideFull" @click="emitPart(addInside(part!, 'tongue'))" />
            </div>
          </div>
          <ul v-if="holdsInside" class="flex flex-col gap-1">
            <li v-for="(inner, index) in part.inside" :key="index">
              <UButton
                color="neutral"
                variant="ghost"
                trailing-icon="i-material-symbols-chevron-right"
                :label="t('edit.fields.inside_part', { paint: paintName(inner), n: index + 1 })"
                class="w-full"
                :ui="{ trailingIcon: 'ms-auto' }"
                @click="emit('enter', index)"
              />
            </li>
          </ul>
        </section>
      </template>
    </template>
  </div>
</template>

<script setup lang="ts">
import { LIMITS, PAINTS } from "#faces";

import { addInside, addNode, moveHandle, removeNode, setClosed, setSmooth } from "~/domain/edit";

import type { Paint, Part, Point } from "#faces";

// The chosen part's numbers (docs/face.md, 「The editor」): the part's openness, paints and width,
// the chosen node's anchor and handles, and what stands inside a closed part. A number is moved
// through the same step a drag takes, so either way the part stays whole on the screen.

const props = defineProps<{
  part: Part | null;
  empty: boolean;
  chosenNode: number | null;
  /** Whether the part stands inside another, whose outline it can be left for. */
  inside: boolean;
}>();
const emit = defineEmits<{ change: [part: Part]; chooseNode: [node: number | null]; enter: [index: number]; leave: [] }>();
const { t } = useI18n();

const NONE = "none";
const AXES = [0, 1] as const;

const node = computed(() => (props.chosenNode === null ? null : (props.part?.nodes[props.chosenNode] ?? null)));
const holdsInside = computed(() => (props.part?.inside ?? []).length > 0);
const insideFull = computed(() => (props.part?.inside ?? []).length >= LIMITS.partsInside);

// A part keeps a fill or a line: "none" is not offered for one while the other is none already.
const paintItems = (otherIsNone: boolean) => [
  { label: t("edit.paint.none"), value: NONE, disabled: otherIsNone },
  ...PAINTS.map((paint) => ({ label: t(`edit.paint.${paint}`), value: paint })),
];
const fillItems = computed(() => paintItems(props.part?.line === null));
const lineItems = computed(() => paintItems(props.part?.fill === null));
const nodeItems = computed(() => (props.part?.nodes ?? []).map((_, index) => ({ label: t("edit.fields.node_n", { n: index + 1 }), value: index })));

const rows = computed(() => {
  const own = node.value;

  return own === null
    ? []
    : [
        { kind: "anchor" as const, value: own.anchor },
        { kind: "coming" as const, value: own.coming },
        { kind: "leaving" as const, value: own.leaving },
      ];
});

const paintName = (part: Part) => t(`edit.paint.${part.fill ?? part.line?.paint ?? NONE}`);

function emitPart(part: Part | null): void {
  if (part !== null) {
    emit("change", part);
  }
}

function setOpenness(closed: boolean): void {
  if (props.part !== null) {
    emitPart(setClosed(props.part, closed));
  }
}

function setFill(value: unknown): void {
  if (props.part !== null && props.part.closed) {
    emitPart({ ...props.part, fill: value === NONE ? null : (value as Paint) });
  }
}

function setLine(value: unknown): void {
  if (props.part !== null) {
    emitPart({ ...props.part, line: value === NONE ? null : { paint: value as Paint, width: props.part.line?.width ?? 2 } });
  }
}

function setWidth(value: number | null): void {
  if (props.part?.line && value !== null) {
    emitPart({ ...props.part, line: { ...props.part.line, width: Math.min(LIMITS.maxWidth, Math.max(LIMITS.minWidth, value)) } });
  }
}

// An anchor's number moves the anchor; a handle's number is its offset, so it moves the handle's end
// to the anchor plus that offset.
function setNumber(kind: "anchor" | "coming" | "leaving", axis: 0 | 1, value: number | null): void {
  const own = node.value;

  if (props.part === null || own === null || props.chosenNode === null || value === null) {
    return;
  }

  const current: Point = kind === "anchor" ? own.anchor : [own.anchor[0] + own[kind][0], own.anchor[1] + own[kind][1]];
  const to: Point = [...current];

  to[axis] = kind === "anchor" ? value : own.anchor[axis] + value;
  emitPart(moveHandle(props.part, { node: props.chosenNode, kind }, to));
}

function moreNode(): void {
  if (props.part !== null) {
    const { part, chosen } = addNode(props.part, props.chosenNode ?? props.part.nodes.length - 1);

    emitPart(part);
    emit("chooseNode", chosen);
  }
}

function dropNode(): void {
  if (props.part !== null && props.chosenNode !== null) {
    emitPart(removeNode(props.part, props.chosenNode));
    emit("chooseNode", null);
  }
}
</script>
