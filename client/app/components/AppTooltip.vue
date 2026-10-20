<template>
  <UTooltip
    v-bind="$attrs"
    v-model:open="open"
    :text="text"
    :disabled="disabled"
    :reference="reference"
    :data-bmo-tooltip="marker"
    @pointerdown="pressDown"
    @pointerup="pressUp"
  >
    <slot />
  </UTooltip>
</template>

<script setup lang="ts">
import { claimPress, pressSpent, strayed, type Point } from "~/domain/long-press";
import { holdTouchTooltip, releaseTouchTooltip } from "~/domain/tooltips";

// The screens' one tooltip (docs/face.md, 「The screens' shared style」). A screen gives it the
// words and whether a press on what it stands on does something else; opening by a held finger,
// the lift that presses nothing, the next touch that puts it away and the placement belong here.
//
// A tooltip that stands on nothing of its own — the layer that shows text cut short — is given a
// reference instead of a target, and its open state is the screen's.

defineOptions({ inheritAttrs: false });

const props = withDefaults(
  defineProps<{
    text: string;
    disabled?: boolean;
    /** Whether a tap on what it stands on does something else. When it does, a held finger opens this. */
    tapActs?: boolean;
    /** What it stands on, when there is no target of its own to wrap. */
    reference?: Element | { getBoundingClientRect: () => DOMRect };
  }>(),
  { disabled: false, tapActs: true, reference: undefined },
);

const open = defineModel<boolean>("open", { default: false });
const slots = useSlots();

// What this tooltip's target carries in the page, so a touch can tell whose press it is.
const marker = useId();
const TRIGGER = `[data-bmo-tooltip="${marker}"]`;

let openedByTouch = false;
let tapAt: Point | null = null;

function putAway(): void {
  openedByTouch = false;
  open.value = false;
  releaseTouchTooltip(putAway);
}

function byTouch(): void {
  openedByTouch = true;
  open.value = true;
  holdTouchTooltip(putAway);
}

watch(open, (standing) => {
  if (!standing) {
    openedByTouch = false;
    releaseTouchTooltip(putAway);
  } else if (openedByTouch) {
    holdTouchTooltip(putAway);
  }
});

onScopeDispose(() => releaseTouchTooltip(putAway));

// A finger held on a target whose tap does something else opens this; the lift presses nothing.
onMounted(() => {
  if (!slots.default) {
    return;
  }

  const release = claimPress((event) => {
    const target = event.target instanceof Element ? event.target.closest(TRIGGER) : null;

    if (target === null) {
      return null;
    }

    return {
      target,
      fire: () => {
        byTouch();

        return true;
      },
    };
  });

  onScopeDispose(release);
});

// A tap opens this where a tap does nothing else. The touch that put a tooltip away is spent and
// opens nothing.
function pressDown(event: PointerEvent): void {
  if (event.pointerType === "touch" && !props.tapActs) {
    tapAt = { x: event.clientX, y: event.clientY };
  }
}

function pressUp(event: PointerEvent): void {
  const from = tapAt;

  tapAt = null;

  if (from === null || event.pointerType !== "touch" || pressSpent(event.pointerId) || open.value) {
    return;
  }

  if (!strayed(from, { x: event.clientX, y: event.clientY })) {
    byTouch();
  }
}
</script>
