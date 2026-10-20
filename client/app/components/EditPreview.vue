<template>
  <section class="flex flex-col gap-2">
    <div class="flex items-center justify-between gap-2">
      <h2 class="text-base font-semibold">{{ t("edit.preview.title") }}</h2>
      <USwitch v-model="talking" :label="t('edit.preview.talk')" />
    </div>
    <div class="preview-body rounded-lg" :style="{ background: document.colors.body }">
      <FaceView :colors="document.colors" :face="face" :mouth="mouth" :label="t('edit.preview.title')" />
    </div>
  </section>
</template>

<script setup lang="ts">
import type { Face, FaceDocument, Part } from "#faces";

// What the display would draw of what is being edited, drawn by the display's own component. It can
// play through the mouth stages on a loop, as speaking would (docs/face.md, 「The editor」).

const props = defineProps<{
  document: FaceDocument;
  face: Face;
  /** A stage to hold the mouth at while the loop is off; null for the face's own mouth. */
  stage: number | null;
}>();
const { t } = useI18n();

const talking = ref(false);
const step = ref(0);
let timer: ReturnType<typeof setInterval> | null = null;

// Up through the stages and back down, as a voice opens and closes the mouth.
const loop = computed(() => {
  const count = props.document.mouths.length;
  const up = Array.from({ length: count }, (_, index) => index);

  return [...up, ...up.slice(1, -1).reverse()];
});

const mouth = computed<Part[] | null>(() => {
  if (talking.value) {
    return props.document.mouths[loop.value[step.value % loop.value.length] ?? 0] ?? null;
  }

  return props.stage === null ? null : (props.document.mouths[props.stage] ?? null);
});

watch(talking, (on) => {
  if (timer !== null) {
    clearInterval(timer);
    timer = null;
  }

  if (on) {
    step.value = 0;
    timer = setInterval(() => (step.value += 1), 120);
  }
});
onBeforeUnmount(() => timer !== null && clearInterval(timer));
</script>

<style scoped>
.preview-body {
  aspect-ratio: 116 / 91;
  padding: 8px;
}
</style>
