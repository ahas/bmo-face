<template>
  <EditorSignIn v-if="editor === null" :login="login" />

  <UDashboardGroup v-else-if="editor !== undefined" storage="local" storage-key="bmo-face-edit" unit="%">
    <UDashboardPanel id="canvas" resizable :default-size="68" :min-size="50" :max-size="80" :ui="{ body: 'p-0 sm:p-0 gap-0 sm:gap-0' }">
      <template #header>
        <UDashboardNavbar :title="t('edit.title')" :toggle="false">
          <template #right>
            <UTooltip :text="t('edit.action.logout')">
              <UButton icon="i-material-symbols-logout" color="neutral" variant="ghost" :aria-label="t('edit.action.logout')" @click="logout" />
            </UTooltip>
          </template>
        </UDashboardNavbar>

        <div class="edit-tabs border-b border-default px-4">
          <UTabs v-model="tab" :items="tabs" :content="false" variant="link" />
        </div>

        <UDashboardToolbar class="edit-toolbar">
          <template #left>
            <UTooltip v-if="outline !== null" :text="t('edit.action.outline')">
              <UButton icon="i-material-symbols-arrow-back" color="neutral" variant="ghost" :aria-label="t('edit.action.outline')" @click="leave" />
            </UTooltip>
            <template v-if="tab === 'faces'">
              <USelect v-model="face" :items="faceItems" :aria-label="t('edit.target.face')" class="w-32" />
              <USelect v-model="group" :items="groupItems" :aria-label="t('edit.target.group')" class="w-28" />
            </template>
            <USelect v-else v-model="stage" :items="stageItems" :aria-label="t('edit.target.stage')" class="w-24" />

            <!-- Wide, the actions stand as icon buttons in groups of three or fewer. -->
            <div class="hidden items-center gap-1 lg:flex">
              <USeparator orientation="vertical" class="h-6" />
              <UDropdownMenu :items="addItems">
                <UTooltip :text="t('edit.action.add_part')">
                  <UButton icon="i-material-symbols-add" color="neutral" variant="ghost" :aria-label="t('edit.action.add_part')" :disabled="!edited" />
                </UTooltip>
              </UDropdownMenu>
              <UTooltip :text="t('edit.action.remove_part')">
                <UButton icon="i-material-symbols-delete-outline" color="neutral" variant="ghost" :aria-label="t('edit.action.remove_part')" :disabled="chosen === null" @click="removePart" />
              </UTooltip>

              <template v-if="tab === 'mouths'">
                <USeparator orientation="vertical" class="h-6" />
                <UTooltip :text="t('edit.action.add_stage')">
                  <UButton icon="i-material-symbols-library-add-outline" color="neutral" variant="ghost" :aria-label="t('edit.action.add_stage')" :disabled="!canAddStage" @click="moreStage" />
                </UTooltip>
                <UTooltip :text="t('edit.action.remove_stage')">
                  <UButton icon="i-material-symbols-layers-clear-outline" color="neutral" variant="ghost" :aria-label="t('edit.action.remove_stage')" :disabled="!canRemoveStage" @click="dropStage" />
                </UTooltip>
              </template>
            </div>

            <!-- Narrow, they gather into one menu, the numbers' drawer with them. -->
            <UDropdownMenu :items="narrowItems" class="lg:hidden">
              <UTooltip :text="t('edit.action.more')">
                <UButton icon="i-material-symbols-more-vert" color="neutral" variant="ghost" :aria-label="t('edit.action.more')" class="lg:hidden" :disabled="!edited" />
              </UTooltip>
            </UDropdownMenu>
          </template>

          <template #right>
            <span v-if="refusal" class="hidden text-sm text-error lg:inline">{{ refusal }}</span>
            <span v-else-if="dirty" class="hidden text-sm text-muted lg:inline">{{ t("edit.status.changed") }}</span>
            <UButton icon="i-material-symbols-save-outline" :label="t('edit.action.save')" :loading="saving" :disabled="!dirty" class="hidden lg:inline-flex" @click="save" />
            <UChip :show="dirty" color="warning" class="lg:hidden">
              <UTooltip :text="dirty ? `${t('edit.action.save')} · ${t('edit.status.changed')}` : t('edit.action.save')">
                <UButton icon="i-material-symbols-save-outline" :aria-label="t('edit.action.save')" :loading="saving" :disabled="!dirty" class="lg:hidden" @click="save" />
              </UTooltip>
            </UChip>
          </template>
        </UDashboardToolbar>

        <p v-if="refusal" class="border-b border-default px-4 py-2 text-sm text-error lg:hidden">{{ refusal }}</p>
      </template>

      <template #body>
        <div class="edit-stage" :style="{ background: edited?.colors.body }">
          <EditCanvas
            v-if="edited"
            :colors="edited.colors"
            :parts="layer"
            :outer="outer"
            :others="others"
            :chosen="chosen"
            :chosen-node="chosenNode"
            :label="targetLabel"
            @change="changeLayer"
            @choose="choose"
          />
          <p v-else-if="loadFailed" class="text-base text-error">{{ t("edit.error.load_failed") }}</p>
          <USkeleton v-else class="edit-skeleton" role="status" :aria-label="t('edit.title')" />
        </div>
      </template>
    </UDashboardPanel>

    <UDashboardPanel id="side" class="hidden lg:flex">
      <template #header>
        <UDashboardNavbar :title="t('edit.fields.title')" :toggle="false" />
      </template>
      <template #body>
        <template v-if="edited">
          <PartFields
            :part="chosenPart"
            :empty="layer.length === 0"
            :chosen-node="chosenNode"
            :inside="outline !== null"
            @change="changePart"
            @choose-node="(node) => (chosenNode = node)"
            @enter="enter"
            @leave="leave"
          />
          <USeparator />
          <EditPreview :document="edited" :face="previewFace" :stage="tab === 'mouths' ? stage : null" />
        </template>
        <USkeleton v-else-if="!loadFailed" class="edit-preview-skeleton" role="status" :aria-label="t('edit.preview.title')" />
        <p v-else class="text-base text-error">{{ t("edit.error.load_failed") }}</p>
      </template>
    </UDashboardPanel>
  </UDashboardGroup>

  <UDrawer v-model:open="drawer" :title="t('edit.fields.title')">
    <template #body>
      <div v-if="edited" class="flex flex-col gap-4">
        <PartFields
          :part="chosenPart"
          :empty="layer.length === 0"
          :chosen-node="chosenNode"
          :inside="outline !== null"
          @change="changePart"
          @choose-node="(node) => (chosenNode = node)"
          @enter="enter"
          @leave="leave"
        />
        <USeparator />
        <EditPreview :document="edited" :face="previewFace" :stage="tab === 'mouths' ? stage : null" />
      </div>
    </template>
  </UDrawer>
</template>

<script setup lang="ts">
import { faceFor, FACE_NAMES, LIMITS, PART_GROUPS } from "#faces";

import { addStage, newPart, partsOf, removeStage, withParts } from "~/domain/edit";
import { useBackClose } from "~/domain/layers";

import type { DropdownMenuItem } from "@nuxt/ui";
import type { FaceName, Part, PartGroup } from "#faces";
import type { EditTarget, NewPart } from "~/domain/edit";

// The expression editor (docs/face.md, 「The editor」): one face's group or one mouth stage at a time
// on the canvas — or the parts inside one of its parts — the chosen part's numbers and the preview
// beside it, and one save for the whole.

const { t } = useI18n();
const { editor, edited, loadFailed, saving, refusal, dirty, login, logout, save } = useEditor();

const tab = ref<"faces" | "mouths">("faces");
const face = ref<FaceName>("기본");
const group = ref<PartGroup>("eyes");
const stage = ref(0);
// The part whose inside is being edited, among the target's parts; null while the target's own
// parts are. `chosen` and `chosenNode` are within what is being edited.
const outline = ref<number | null>(null);
const chosen = ref<number | null>(null);
const chosenNode = ref<number | null>(null);
const drawer = ref(false);

const tabs = computed(() => [
  { label: t("edit.tab.faces"), value: "faces" },
  { label: t("edit.tab.mouths"), value: "mouths" },
]);
const faceItems = computed(() => FACE_NAMES.map((name) => ({ label: t(`face.name.${name}`), value: name })));
const groupItems = computed(() => PART_GROUPS.map((name) => ({ label: t(`edit.group.${name}`), value: name })));
const stageItems = computed(() => (edited.value?.mouths ?? []).map((_, index) => ({ label: t("edit.stage", { n: index + 1 }), value: index })));
const newItems = (): DropdownMenuItem[] => [
  { label: t("edit.action.add_oval"), icon: "i-material-symbols-circle-outline", disabled: layerFull.value, onSelect: () => addPart("oval") },
  { label: t("edit.action.add_line"), icon: "i-material-symbols-gesture", disabled: layerFull.value, onSelect: () => addPart("line") },
];
const addItems = computed(() => [newItems()]);

const canAddStage = computed(() => edited.value !== null && edited.value.mouths.length < LIMITS.maxStages);
const canRemoveStage = computed(() => edited.value !== null && edited.value.mouths.length > LIMITS.minStages);

// On a narrow screen every action of the toolbar stands in one menu, in the order they are reached for.
const narrowItems = computed((): DropdownMenuItem[][] => [
  [
    { label: t("edit.action.fields"), icon: "i-material-symbols-tune", onSelect: () => (drawer.value = true) },
  ],
  [...newItems(), { label: t("edit.action.remove_part"), icon: "i-material-symbols-delete-outline", disabled: chosen.value === null, onSelect: removePart }],
  ...(tab.value === "mouths"
    ? [
        [
          { label: t("edit.action.add_stage"), icon: "i-material-symbols-library-add-outline", disabled: !canAddStage.value, onSelect: moreStage },
          { label: t("edit.action.remove_stage"), icon: "i-material-symbols-layers-clear-outline", disabled: !canRemoveStage.value, onSelect: dropStage },
        ],
      ]
    : []),
]);

const target = computed<EditTarget>(() => (tab.value === "faces" ? { tab: "faces", face: face.value, group: group.value } : { tab: "mouths", stage: stage.value }));
const parts = computed<Part[]>(() => (edited.value === null ? [] : partsOf(edited.value, target.value)));
// The part whose inside is being edited, and what is being edited: its inside, or the target's parts.
const outer = computed<Part | null>(() => (outline.value === null ? null : (parts.value[outline.value] ?? null)));
const layer = computed<Part[]>(() => (outline.value === null ? parts.value : (outer.value?.inside ?? [])));
const layerFull = computed(() => layer.value.length >= (outline.value === null ? LIMITS.partsPerGroup : LIMITS.partsInside));
const chosenPart = computed(() => (chosen.value === null ? null : (layer.value[chosen.value] ?? null)));
const targetLabel = computed(() => {
  const named = tab.value === "faces" ? `${t(`face.name.${face.value}`)} · ${t(`edit.group.${group.value}`)}` : t("edit.stage", { n: stage.value + 1 });

  return outline.value === null ? named : `${named} · ${t("edit.fields.inside")}`;
});

// The rest of the face behind what is being edited, each part in its own paints: the face's other
// groups, or, for a mouth stage, the 기본 face without its mouth; and, inside a part, the target's
// other parts.
const others = computed<Part[]>(() => {
  const document = edited.value;

  if (document === null) {
    return [];
  }

  const shown = tab.value === "faces" ? faceFor(document, face.value) : faceFor(document, "기본");
  const skipped: PartGroup = tab.value === "faces" ? group.value : "mouth";
  const siblings = outline.value === null ? [] : parts.value.filter((_, index) => index !== outline.value);

  return [...PART_GROUPS.filter((name) => name !== skipped).flatMap((name) => shown[name]), ...siblings];
});
const previewFace = computed(() => (edited.value === null ? null : faceFor(edited.value, tab.value === "faces" ? face.value : "기본"))!);

// A new target starts at its own parts, with nothing chosen.
watch(target, () => {
  outline.value = null;
  chosen.value = null;
  chosenNode.value = null;
});

// Editing inside a part is a place the screen has gone into, and the back gesture leaves it.
useBackClose(
  computed(() => outline.value !== null),
  () => leave(),
);

function changeParts(next: Part[]): void {
  if (edited.value !== null) {
    edited.value = withParts(edited.value, target.value, next);
  }
}

function changeLayer(next: Part[]): void {
  const at = outline.value;

  if (at === null) {
    changeParts(next);

    return;
  }

  changeParts(
    parts.value.map((part, index) => {
      if (index !== at) {
        return part;
      }

      const { inside: _old, ...rest } = part;

      return next.length > 0 ? { ...rest, inside: next } : rest;
    }),
  );
}

function changePart(part: Part): void {
  if (chosen.value !== null) {
    changeLayer(layer.value.map((each, index) => (index === chosen.value ? part : each)));
  }
}

function choose(part: number, node: number | null): void {
  if (part !== chosen.value) {
    chosenNode.value = null;
  }

  chosen.value = part;
  chosenNode.value = node ?? chosenNode.value;
}

// Into the parts inside the chosen part, and back out to it.
function enter(index: number): void {
  if (chosen.value !== null) {
    outline.value = chosen.value;
    chosen.value = index;
    chosenNode.value = null;
  }
}

function leave(): void {
  if (outline.value !== null) {
    chosen.value = outline.value;
    outline.value = null;
    chosenNode.value = null;
  }
}

function addPart(kind: NewPart): void {
  if (!layerFull.value) {
    changeLayer([...layer.value, newPart(kind)]);
    choose(layer.value.length - 1, null);
  }
}

function removePart(): void {
  if (chosen.value !== null) {
    changeLayer(layer.value.filter((_, index) => index !== chosen.value));
    chosen.value = null;
    chosenNode.value = null;
  }
}

function moreStage(): void {
  if (edited.value !== null) {
    edited.value = addStage(edited.value, stage.value);
    stage.value += 1;
  }
}

function dropStage(): void {
  if (edited.value !== null) {
    edited.value = removeStage(edited.value, stage.value);
    stage.value = Math.min(stage.value, edited.value.mouths.length - 1);
  }
}

useHead({ title: () => t("edit.title") });
</script>

<style scoped>
.edit-tabs {
  display: flex;
  align-items: flex-end;
}

.edit-stage {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
}

.edit-preview-skeleton {
  width: 100%;
  aspect-ratio: 116 / 91;
}

.edit-skeleton {
  width: 100%;
  max-height: 100%;
  aspect-ratio: 108 / 83;
}
</style>
