<template>
  <!-- Every word these screens have for their reader stands on the one island at the top of the
       screen (`~/domain/island`), so the component library's own toaster is not stood up at all. -->
  <UApp :locale="LIBRARY_LOCALES[locale] ?? ko" :tooltip="TOOLTIPS" :toaster="null">
    <NuxtPage />

    <MessageIsland />
    <ClippedTextTooltip />
  </UApp>
</template>

<script setup lang="ts">
import { en, ja, ko, zh_cn } from "@nuxt/ui/locale";

// The component library's own words, in the locale the screen speaks.
const LIBRARY_LOCALES: Record<string, typeof ko> = { ko, en, ja, "zh-CN": zh_cn };

const { locale } = useI18n();

// A tooltip answers hover and the keyboard. Focus handed back by a closing sheet to the button that
// opened it is neither, and on a phone it would leave the button's name standing over the screen.
const TOOLTIPS = { ignoreNonKeyboardFocus: true };
</script>
