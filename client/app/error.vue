<template>
  <!-- bmo-ui's error page in place of the framework's (docs/face.md, 「The screens' shared style」): the layer's screen,
       imported whole, inside an app root of its own, since the app's root is not drawn while it stands. -->
  <UApp :locale="LIBRARY_LOCALES[locale] ?? ko" :tooltip="TOOLTIPS" :toaster="null">
    <ErrorScreen :status="error.statusCode" :message="error.message" :stack="error.stack" />
  </UApp>
</template>

<script setup lang="ts">
import { en, ja, ko, zh_cn } from "@nuxt/ui/locale";
import ErrorScreen from "bmo-ui/app/components/ErrorScreen.vue";

import type { NuxtError } from "#app";

defineProps<{ error: NuxtError }>();

// The component library's own words, in the locale the screen speaks.
const LIBRARY_LOCALES: Record<string, typeof ko> = { ko, en, ja, "zh-CN": zh_cn };

const { locale } = useI18n();

// A tooltip answers hover and the keyboard alone, as on every screen of the face.
const TOOLTIPS = { ignoreNonKeyboardFocus: true };
</script>
