<template>
  <!-- The standalone screens' copy of bmo-ui's ChoiceSelect (docs/face.md, 「The screens' shared style」):
       the console bundle takes the layer's own in its place, so this keeps the layer's rule, words and
       look for the lists these screens hand it, flat lists of their own choices. A list searches once
       it holds more than seven choices; the field stands at the top of the opened list with the pack's
       words for a search, takes the focus unless a finger opened it, and matches what each choice
       shows, its hits marked where they stand. An empty list says whether nothing matched or nothing
       is there, and the control names the chosen choice from the whole list. The list opens against
       the control's leading edge. Everything else it is handed is the library's select menu's. -->
  <USelectMenu
    v-bind="$attrs"
    v-model:search-term="words"
    :model-value="props.modelValue"
    :value-key="props.valueKey"
    :items="shown"
    :label-key="props.labelKey"
    :search-input="searchInput"
    :content="content"
    ignore-filter
  >
    <template v-for="name in passedSlots" #[name]="scope" :key="name">
      <slot :name="name" v-bind="scope ?? {}" />
    </template>

    <template #item-label="{ item }">
      <span><template v-for="(part, index) in partsOf(item)" :key="index"><mark v-if="part.hit" class="rounded-sm bg-warning/25 text-highlighted">{{ part.text }}</mark><template v-else>{{ part.text }}</template></template></span>
    </template>

    <template v-if="!slots.default" #default="{ ui }">
      <span v-if="shownValue !== undefined" data-slot="value" :class="ui.value()">{{ shownValue }}</span>
      <span v-else data-slot="placeholder" :class="ui.placeholder()">{{ attrs.placeholder ?? " " }}</span>
    </template>

    <template #empty>
      {{ emptyWords }}
    </template>
  </USelectMenu>
</template>

<script setup lang="ts">
import { choiceLabel, choiceParts, chosenLabel, narrowChoices, searchesChoices } from "~/domain/choices";

import type { SelectMenuItem, SelectMenuProps } from "@nuxt/ui";
import type { ChoicePart } from "~/domain/choices";

interface Props extends /* @vue-ignore */ Omit<SelectMenuProps, "items" | "modelValue" | "valueKey" | "searchInput" | "searchTerm" | "ignoreFilter" | "labelKey"> {
  // The value chosen.
  modelValue?: unknown;
  // The field of a choice holding its value; the choice itself where unset.
  valueKey?: string;
  // The choices, one flat list.
  items: SelectMenuItem[];
  // The screen's own word for its list when it holds nothing, where the wording guide gives one.
  empty?: string;
  // The field of a choice holding its label.
  labelKey?: string;
}

defineOptions({ inheritAttrs: false });

const props = withDefaults(defineProps<Props>(), {
  modelValue: undefined,
  valueKey: undefined,
  empty: undefined,
  labelKey: "label",
});

// A touch screen's pointer: a finger opening the list leaves the keyboard down until the field is pressed.
const TOUCH_QUERY = "(pointer: coarse)";

const slots = useSlots();
const attrs = useAttrs();
const { t, locale } = useI18n();
const appConfig = useAppConfig();
const touch = useMediaQuery(TOUCH_QUERY);

// The slots the select draws itself; every other slot of the library's select menu is the caller's.
const OWN_SLOTS = new Set(["item-label", "empty"]);
const passedSlots = computed(() => Object.keys(slots).filter((name) => !OWN_SLOTS.has(name)));

// The words typed into the search; the library clears them as the list closes.
const words = ref("");

const searchInput = computed(() =>
  searchesChoices(props.items) ? { placeholder: t("common.placeholder.search"), icon: appConfig.ui.icons.search, ...(touch.value ? { autofocus: false } : {}) } : false,
);

const shown = computed(() => narrowChoices(props.items, words.value, props.labelKey, locale.value));

// The list opens against the control's leading edge unless the screen says where.
const content = computed(() => ({ align: "start" as const, ...(attrs.content as object | undefined) }));

// A choice's label as the list draws it, cut where the search found the words typed.
function partsOf(item: SelectMenuItem): ChoicePart[] {
  return choiceParts(choiceLabel(item, props.labelKey), words.value, locale.value);
}

// What the control shows as chosen, named from the whole list rather than the narrowed one the library
// is handed, so typing a search never leaves it showing the raw value.
const shownValue = computed(() => chosenLabel(props.items, props.modelValue, props.valueKey, props.labelKey));

const emptyWords = computed(() => (words.value.trim() !== "" ? t("common.status.no_match") : (props.empty ?? t("common.status.no_choices"))));
</script>
