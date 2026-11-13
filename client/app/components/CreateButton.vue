<template>
  <!-- The standalone screens' copy of bmo-ui's CreateButton (docs/face.md, 「The screens' shared style」):
       the console bundle takes the layer's own in its place, so this keeps the layer's shapes, key and
       room. On a wide screen a grey ghost icon in its own cell of the row the screen's actions stand
       in, its tooltip naming what it creates and then its key; on a phone a primary solid icon button
       floating at the bottom right, and nothing of it in the row.

       Where its cell stands is the host's, through a class or a style; everything else handed to it,
       a press or what a menu wrapped round it puts on its trigger, lands on the button. -->
  <div v-if="!narrow" :class="['flex shrink-0 items-center', $attrs.class]" :style="$attrs.style as StyleValue">
    <AppTooltip :text="props.label" :kbds="kbds">
      <UButton
        ref="button"
        v-bind="forButton()"
        :icon="ADD_ICON"
        color="neutral"
        variant="ghost"
        square
        :aria-label="props.label"
        :disabled="props.disabled"
        :aria-keyshortcuts="key ?? undefined"
      />
    </AppTooltip>
  </div>
  <Teleport v-else to="body">
    <div data-screen-foot class="badge-ink fixed end-4 bottom-[calc(1rem+var(--bmo-safe-bottom))]">
      <AppTooltip :text="props.label">
        <UButton
          ref="button"
          v-bind="forButton()"
          :icon="ADD_ICON"
          color="primary"
          variant="solid"
          size="xl"
          square
          :aria-label="props.label"
          :disabled="props.disabled"
          :aria-keyshortcuts="key ?? undefined"
        />
      </AppTooltip>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { layerCount } from "~/domain/layers";

import type { ComponentPublicInstance, StyleValue } from "vue";

interface Props {
  /** What it creates, in the host's words: its name, heard and shown alike. */
  label: string;
  /** The key that presses it; null for none. */
  shortcut?: string | null;
  disabled?: boolean;
}

defineOptions({ inheritAttrs: false });

const props = withDefaults(defineProps<Props>(), { shortcut: "n", disabled: false });

const ADD_ICON = "i-material-symbols-add";

const attrs = useAttrs();
const narrow = useNarrow();
const button = useTemplateRef<ComponentPublicInstance>("button");

// The key is named once, as the button is set up.
const key = props.shortcut;
const kbds = key === null ? undefined : [key];

// What is handed to it for the button: everything but where its cell stands.
function forButton(): Record<string, unknown> {
  const { class: _class, style: _style, ...rest } = attrs;

  return rest;
}

function element(): HTMLElement | null {
  return (button.value?.$el as HTMLElement | undefined) ?? null;
}

// The key does what a press does, and only where a press could land: the button enabled and nothing
// floating over the screen. The library's shortcuts already pass over a key typed into a field.
function pressByKey(): void {
  if (props.disabled || layerCount() > 0) {
    return;
  }

  element()?.click();
}

if (key !== null) {
  defineShortcuts({ [key]: pressByKey });
}

defineExpose({ element });
</script>
