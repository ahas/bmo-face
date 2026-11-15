<template>
  <section class="flex flex-col gap-2">
    <div class="flex items-center justify-between gap-2">
      <h2 class="text-base font-semibold">{{ t("face.edit.preview.title") }}</h2>
      <div class="flex items-center gap-4">
        <USwitch v-model="blinking" :label="t('face.edit.preview.blink')" />
        <USwitch v-model="talking" :label="t('face.edit.preview.talk')" />
      </div>
    </div>
    <div class="preview-body rounded-lg" :style="{ background: document.colors.body }">
      <FaceView :drawing="drawing" :screen="document.colors.screen" :label="t('face.edit.preview.title')" />
    </div>
    <div class="grid grid-cols-2 gap-4">
      <UFormField v-for="axis in AXES" :key="axis" :label="t(axis === 0 ? 'face.edit.preview.gaze_x' : 'face.edit.preview.gaze_y')">
        <USlider v-model="gaze[axis]" :min="-1" :max="1" :step="0.1" />
      </UFormField>
    </div>
  </section>
</template>

<script setup lang="ts">
import { FACTS, sampleLoudness } from "#core";
import { faceFor, FALLBACK_FACE } from "#faces";

import type { FaceDocument, FaceName, Part } from "#faces";

// The face being edited as a display would draw it (docs/face.md, 「The editor」): the core's own scene
// of the document, pinned to the face, on the held clock. It blinks as a display does unless that is
// turned off, speaks a sampled voice with the talking mouth set while talking is on, and looks where its
// two sliders turn the gaze. A talking mouth being edited stands in the fallback face's mouth.

const props = defineProps<{
  document: FaceDocument;
  face: FaceName;
  /** A talking mouth to show in the fallback face; null for the face's own mouth. */
  mouth: Part[] | null;
}>();
const { t } = useI18n();

const AXES = [0, 1] as const;
/** How long one sampled sentence runs, in frames, and the seed it is drawn from. */
const SAMPLE_FRAMES = 240;

const shown = computed<FaceDocument>(() => {
  if (props.mouth === null) {
    return props.document;
  }

  const fallback = faceFor(props.document, FALLBACK_FACE);

  return { ...props.document, faces: { ...props.document.faces, [FALLBACK_FACE]: { ...fallback, mouth: props.mouth } } };
});
const { drawing, clock, tell } = useFaceScene(shown, 1);

const blinking = ref(true);
const talking = ref(false);
const gaze = reactive<[number, number]>([0, 0]);
let sentence = 0;
let next: ReturnType<typeof setTimeout> | null = null;

/** One sampled sentence, and the next after it while talking stays on. */
function speakOnce(): void {
  const heard = clock.frame() + FACTS.lead + 1;

  sentence += 1;
  tell((scene) => scene.speak({ loudness: sampleLoudness(sentence, SAMPLE_FRAMES), seed: sentence }, props.face, heard));
  next = setTimeout(speakOnce, clock.timeOf(heard + SAMPLE_FRAMES) - performance.now());
}

function quiet(): void {
  if (next !== null) {
    clearTimeout(next);
    next = null;
  }

  tell((scene, frame) => scene.stop(frame));
}

watch(
  () => props.face,
  (face) => tell((scene) => scene.pin(face)),
  { immediate: true },
);
watch(blinking, (on) => tell((scene) => scene.setBlinking(on)));
watch(talking, (on) => (on ? speakOnce() : quiet()));
watch(gaze, ([x, y]) => tell((scene, frame) => scene.gaze(x, y, frame)));
onBeforeUnmount(quiet);
</script>

<style scoped>
.preview-body {
  aspect-ratio: 116 / 91;
  padding: 8px;
}
</style>
