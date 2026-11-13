<template>
  <EditorSignIn v-if="editor === null" :login="login" />

  <!-- Under a console that offers its title bar, the editor's two panels stand in the console's own group and draw
       no bar (docs/face.md, 「The console screen」); on its own page it is a group of its own with its bars. -->
  <EditFrame v-else-if="editor !== undefined" :offered="barOffered">
    <UDashboardPanel id="canvas" resizable :default-size="68" :min-size="50" :max-size="80" :ui="{ body: 'p-0 sm:p-0 gap-0 sm:gap-0' }">
      <template #header>
        <UDashboardNavbar v-if="!barOffered" :title="t('face.edit.title')" :toggle="false">
          <template #right>
            <AppTooltip v-if="road.session" :text="t('face.edit.action.logout')">
              <UButton icon="i-material-symbols-logout" color="neutral" variant="ghost" :aria-label="t('face.edit.action.logout')" @click="logout" />
            </AppTooltip>
          </template>
        </UDashboardNavbar>

        <div class="edit-tabs border-b border-default px-4">
          <UTabs v-model="tab" :items="tabs" :content="false" variant="link" />
        </div>

        <UDashboardToolbar class="edit-toolbar">
          <template #left>
            <AppTooltip v-if="outline !== null" :text="t('face.edit.action.outline')">
              <UButton size="sm" icon="i-material-symbols-arrow-back" color="neutral" variant="ghost" :aria-label="t('face.edit.action.outline')" @click="leave" />
            </AppTooltip>
            <template v-if="tab === 'faces'">
              <USelect v-model="face" size="sm" :items="faceItems" :aria-label="t('face.edit.target.face')" class="w-32 min-w-0" />
              <USelect v-model="group" size="sm" :items="groupItems" :aria-label="t('face.edit.target.group')" class="w-28 min-w-0" />
            </template>
            <USelect v-else v-model="stage" size="sm" :items="stageItems" :aria-label="t('face.edit.target.stage')" class="w-24 min-w-0" />

            <!-- Wide, the actions stand as icon buttons in groups of three or fewer. -->
            <div class="hidden items-center gap-1 lg:flex">
              <USeparator orientation="vertical" class="h-6" />
              <AppTooltip :text="t('face.edit.action.remove_part')">
                <UButton size="sm" icon="i-material-symbols-delete-outline" color="neutral" variant="ghost" :aria-label="t('face.edit.action.remove_part')" :disabled="chosen === null" @click="removePart" />
              </AppTooltip>

              <template v-if="tab === 'mouths'">
                <USeparator orientation="vertical" class="h-6" />
                <AppTooltip :text="t('face.edit.action.remove_stage')">
                  <UButton size="sm" icon="i-material-symbols-layers-clear-outline" color="neutral" variant="ghost" :aria-label="t('face.edit.action.remove_stage')" :disabled="!canRemoveStage" @click="dropStage" />
                </AppTooltip>
              </template>
            </div>

            <!-- Narrow, they gather into one menu, the numbers' drawer with them. -->
            <UDropdownMenu :items="narrowItems" class="lg:hidden">
              <AppTooltip :text="t('face.edit.action.more')">
                <UButton size="sm" icon="i-material-symbols-more-vert" color="neutral" variant="ghost" :aria-label="t('face.edit.action.more')" class="lg:hidden" :disabled="!edited" />
              </AppTooltip>
            </UDropdownMenu>
          </template>

          <template #right>
            <span v-if="refusal" class="hidden text-sm text-error lg:inline">{{ refusal }}</span>
            <span v-else-if="dirty" class="hidden text-sm text-muted lg:inline">{{ t("face.edit.status.changed") }}</span>
            <UButton size="sm" icon="i-material-symbols-save-outline" :label="t('face.edit.action.save')" :loading="saving" :disabled="!dirty" class="hidden lg:inline-flex" @click="save" />
            <UChip :show="dirty" color="warning" class="lg:hidden">
              <AppTooltip :text="dirty ? `${t('face.edit.action.save')} · ${t('face.edit.status.changed')}` : t('face.edit.action.save')">
                <UButton size="sm" icon="i-material-symbols-save-outline" :aria-label="t('face.edit.action.save')" :loading="saving" :disabled="!dirty" class="lg:hidden" @click="save" />
              </AppTooltip>
            </UChip>

            <!-- What the editor makes, made from its one create button at the toolbar's right end. -->
            <UDropdownMenu :items="createItems" :content="createMenu">
              <CreateButton ref="createButton" size="sm" :label="createLabel" :disabled="!edited" />
            </UDropdownMenu>
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
          <p v-else-if="loadFailed" class="text-base text-error">{{ t("face.edit.error.load_failed") }}</p>
          <USkeleton v-else class="edit-skeleton" role="status" :aria-label="t('face.edit.title')" />
        </div>
      </template>
    </UDashboardPanel>

    <UDashboardPanel id="side" class="hidden lg:flex">
      <template #header>
        <UDashboardNavbar v-if="!barOffered" :title="t('face.edit.fields.title')" :toggle="false" />
        <p v-else class="flex h-(--ui-header-height) shrink-0 items-center border-b border-default px-4 text-sm font-medium text-highlighted">
          {{ t("face.edit.fields.title") }}
        </p>
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
        <USkeleton v-else-if="!loadFailed" class="edit-preview-skeleton" role="status" :aria-label="t('face.edit.preview.title')" />
        <p v-else class="text-base text-error">{{ t("face.edit.error.load_failed") }}</p>
      </template>
    </UDashboardPanel>
  </EditFrame>

  <UDrawer v-model:open="drawer" :title="t('face.edit.fields.title')">
    <template #close="{ ui }">
      <AppTooltip :text="t('common.action.close')">
        <UButton icon="i-material-symbols-close" color="neutral" variant="ghost" :aria-label="t('common.action.close')" :class="ui.close()" />
      </AppTooltip>
    </template>
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

import CreateButton from "~/components/CreateButton.vue";
import { addInside, addStage, newPart, partsOf, removeStage, withParts } from "~/domain/edit";
import { useBackClose } from "~/domain/layers";

import type { DropdownMenuItem } from "@nuxt/ui";
import type { FaceName, Part, PartGroup } from "#faces";
import type { EditTarget, NewInside, NewPart } from "~/domain/edit";
import type { EditorRoad } from "~/composables/useEditor";

// The expression editor (docs/face.md, 「The editor」): one face's group or one mouth stage at a time
// on the canvas — or the parts inside one of its parts — the chosen part's numbers and the preview
// beside it, and one save for the whole. The page's own route reaches the server behind the editor's
// login; the console screen hands it the console's road, which needs none (「The console screen」).

const props = defineProps<{ road?: EditorRoad }>();

// Whether the console this screen stands in offers its title bar, under bmo-web's key for it.
const barOffered = inject("bmo-console-title-bar", null) !== null;
const road = props.road ?? sessionRoad();
const { t } = useI18n();
const { editor, edited, loadFailed, saving, refusal, dirty, login, logout, save } = useEditor(road);

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
const createButton = useTemplateRef<{ element: () => HTMLElement | null }>("createButton");

const tabs = computed(() => [
  { label: t("face.edit.tab.faces"), value: "faces" },
  { label: t("face.edit.tab.mouths"), value: "mouths" },
]);
const faceItems = computed(() => FACE_NAMES.map((name) => ({ label: t(`face.name.${name}`), value: name })));
const groupItems = computed(() => PART_GROUPS.map((name) => ({ label: t(`face.edit.group.${name}`), value: name })));
const stageItems = computed(() => (edited.value?.mouths ?? []).map((_, index) => ({ label: t("face.edit.stage", { n: index + 1 }), value: index })));
const canAddStage = computed(() => edited.value !== null && edited.value.mouths.length < LIMITS.maxStages);
const canRemoveStage = computed(() => edited.value !== null && edited.value.mouths.length > LIMITS.minStages);

// What the create button makes (docs/face.md, 「The editor」): a part of what is being edited; inside a
// chosen closed part of the target's own, a teeth band or a tongue; and, in the mouth stage tab, a stage.
const createLabel = computed(() => (tab.value === "faces" ? t("face.edit.action.add_part") : t("face.edit.action.add_part_stage")));
const createItems = computed((): DropdownMenuItem[][] => {
  const own = chosenPart.value;
  const groups: DropdownMenuItem[][] = [
    [
      { label: t("face.edit.action.add_oval"), icon: "i-material-symbols-circle-outline", disabled: layerFull.value, onSelect: () => addPart("oval") },
      { label: t("face.edit.action.add_line"), icon: "i-material-symbols-gesture", disabled: layerFull.value, onSelect: () => addPart("line") },
    ],
  ];

  if (outline.value === null && own !== null && own.closed) {
    const full = (own.inside ?? []).length >= LIMITS.partsInside;

    groups.push([
      { type: "label", label: t("face.edit.fields.inside") },
      { label: t("face.edit.action.add_teeth"), icon: "i-material-symbols-dentistry-outline", disabled: full, onSelect: () => addPartInside("teeth") },
      { label: t("face.edit.action.add_tongue"), icon: "i-material-symbols-sentiment-very-satisfied-outline", disabled: full, onSelect: () => addPartInside("tongue") },
    ]);
  }

  if (tab.value === "mouths") {
    groups.push([{ label: t("face.edit.action.add_stage"), icon: "i-material-symbols-library-add-outline", disabled: !canAddStage.value, onSelect: moreStage }]);
  }

  return groups;
});

// The menu stands on the create button itself and gives the focus back to it as it closes: what the menu
// takes for its trigger is the button's cell on a wide screen, which takes no focus, and on a phone,
// where the button floats in the page's body, nothing of the button at all. The menu is handed one
// anchor for good, as the library reads which settings it passes on only as the menu is first set up,
// and the anchor reads the button wherever it stands at the moment the menu places itself, since the
// button changes its shape with the screen's width.
const createAnchor = {
  getBoundingClientRect: (): DOMRect => createButton.value?.element()?.getBoundingClientRect() ?? new DOMRect(),
  get contextElement(): HTMLElement | undefined {
    return createButton.value?.element() ?? undefined;
  },
};
const createMenu = {
  align: "end" as const,
  reference: createAnchor,
  onCloseAutoFocus: (event: Event) => {
    event.preventDefault();
    createButton.value?.element()?.focus();
  },
};

// On a narrow screen every other action of the toolbar stands in one menu, in the order they are reached for.
const narrowItems = computed((): DropdownMenuItem[][] => [
  [{ label: t("face.edit.action.fields"), icon: "i-material-symbols-tune", onSelect: () => (drawer.value = true) }],
  [{ label: t("face.edit.action.remove_part"), icon: "i-material-symbols-delete-outline", disabled: chosen.value === null, onSelect: removePart }],
  ...(tab.value === "mouths"
    ? [[{ label: t("face.edit.action.remove_stage"), icon: "i-material-symbols-layers-clear-outline", disabled: !canRemoveStage.value, onSelect: dropStage }]]
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
  const named = tab.value === "faces" ? `${t(`face.name.${face.value}`)} · ${t(`face.edit.group.${group.value}`)}` : t("face.edit.stage", { n: stage.value + 1 });

  return outline.value === null ? named : `${named} · ${t("face.edit.fields.inside")}`;
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

function addPartInside(kind: NewInside): void {
  if (chosenPart.value !== null) {
    changePart(addInside(chosenPart.value, kind));
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

useHead({ title: () => t("face.edit.title") });
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
  padding: 16px 16px calc(16px + var(--bmo-create-room));
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
