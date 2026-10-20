<template>
  <!-- A number box with its two step buttons laid out by the screen it is read on (docs/face.md, 「The
       editor」): on a wide screen Nuxt UI's vertical field, an up arrow over a down arrow stacked at
       its end; on a narrow one its horizontal field, − at its start and + at its end. Each step
       button names itself in a tooltip, and the figures keep their width as a drag moves them. -->
  <UInputNumber v-bind="$attrs" :orientation="narrow ? 'horizontal' : 'vertical'" :ui="{ base: 'tabular-nums' }">
    <template #increment>
      <UTooltip :text="t('common.aria.increase')">
        <UButton :key="String(narrow)" :icon="narrow ? appConfig.ui.icons.plus : appConfig.ui.icons.chevronUp" color="neutral" variant="link" :aria-label="t('common.aria.increase')" />
      </UTooltip>
    </template>

    <template #decrement>
      <UTooltip :text="t('common.aria.decrease')">
        <UButton :key="String(narrow)" :icon="narrow ? appConfig.ui.icons.minus : appConfig.ui.icons.chevronDown" color="neutral" variant="link" :aria-label="t('common.aria.decrease')" />
      </UTooltip>
    </template>
  </UInputNumber>
</template>

<script setup lang="ts">
defineOptions({ inheritAttrs: false });

const { t } = useI18n();
const appConfig = useAppConfig();
// A button is made anew when the screen changes its icon, since the icon library writes the
// drawing it was still loading for the old name under the new one.
const narrow = useNarrow();
</script>
