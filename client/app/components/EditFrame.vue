<template>
  <!-- What the editor's two panels stand in (docs/face.md, 「The console screen」): on its own page a dashboard group of
       its own; under a console that offers its title bar, a row inside the console's own group. -->
  <UDashboardGroup v-if="!props.offered" storage="local" storage-key="bmo-face-edit" unit="%">
    <slot />
  </UDashboardGroup>
  <div v-else class="flex min-h-0 min-w-0 flex-1">
    <slot />
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{ offered: boolean }>();
</script>
