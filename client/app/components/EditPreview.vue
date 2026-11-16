<template>
  <section class="flex flex-col gap-2">
    <div class="flex items-center justify-between gap-2">
      <h2 class="text-base font-semibold">{{ t("face.edit.preview.title") }}</h2>
      <USwitch v-model="blinking" :label="t('face.edit.preview.blink')" />
    </div>
    <div class="preview-body rounded-lg" :style="{ background: document.colors.body }">
      <FaceView :drawing="drawing" :screen="document.colors.screen" :label="t('face.edit.preview.title')" />
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <UButton size="sm" icon="i-material-symbols-graphic-eq" color="neutral" variant="outline" :label="t('face.edit.preview.sample')" @click="speakSample" />
      <UButton size="sm" icon="i-material-symbols-audio-file-outline" color="neutral" variant="outline" :label="t('face.edit.preview.file')" @click="pickFile" />
      <UButton size="sm" icon="i-material-symbols-stop" color="neutral" variant="ghost" :label="t('face.edit.preview.stop')" :disabled="!speaking" @click="quiet" />
      <input ref="file" type="file" accept="audio/*" class="hidden" @change="speakFile" />
    </div>
    <p v-if="fileFailed" class="text-sm text-error">{{ t("face.edit.preview.file_failed") }}</p>
    <SpectrumStrip v-if="heard.length > 0" :heard="heard" :poses="planned" :frame="shownFrame" />
    <div class="grid grid-cols-2 gap-4">
      <UFormField v-for="axis in AXES" :key="axis" :label="t(axis === 0 ? 'face.edit.preview.gaze_x' : 'face.edit.preview.gaze_y')">
        <USlider v-model="gaze[axis]" :min="-1" :max="1" :step="0.1" />
      </UFormField>
    </div>
  </section>
</template>

<script setup lang="ts">
import { FACTS, hear, poses, sampleVoice } from "#core";
import { faceFor, FALLBACK_FACE } from "#faces";

import { AUDIO_RATE } from "~/composables/useSpeaker";

import type { Heard, Pose } from "#core";
import type { FaceDocument, FaceName, Part } from "#faces";

// The face being edited as a display would draw it (docs/face.md, 「The editor」): the core's own scene
// of the document, pinned to the face, on the held clock. It blinks as a display does unless that is
// turned off, and looks where its two sliders turn the gaze. It speaks the core's sampled voice or a
// sound file aloud, heard by the core as a display hears a briefing, with the document as it is being
// edited; the strip beneath draws what is heard and what the tuning makes of it. An anchor being edited
// stands in the fallback face's mouth.

const props = defineProps<{
  document: FaceDocument;
  face: FaceName;
  /** An anchor's drawing to show in the fallback face; null for the face's own mouth. */
  mouth: Part[] | null;
}>();
const { t } = useI18n();

const AXES = [0, 1] as const;
/** How many frames the first drawing waits after the sound is ready, so its own frame is not already past. */
const SETTLE_FRAMES = 1;

const shown = computed<FaceDocument>(() => {
  if (props.mouth === null) {
    return props.document;
  }

  const fallback = faceFor(props.document, FALLBACK_FACE);

  return { ...props.document, faces: { ...props.document.faces, [FALLBACK_FACE]: { ...fallback, mouth: props.mouth } } };
});
const { drawing, clock, tell } = useFaceScene(shown, 1);

const blinking = ref(true);
const gaze = reactive<[number, number]>([0, 0]);
const file = useTemplateRef<HTMLInputElement>("file");
const fileFailed = ref(false);
const heard = shallowRef<Heard[]>([]);
const planned = shallowRef<Pose[]>([]);
const speaking = ref(false);
/** The frame of the sound whose pose the face shows now; null while nothing plays. */
const shownFrame = ref<number | null>(null);

let context: AudioContext | null = null;
let source: AudioBufferSourceNode | null = null;
let following: number | null = null;
// Each play is numbered, so a stop or a newer play ends the older one.
let playing = 0;

function audio(): AudioContext {
  context ??= new AudioContext({ sampleRate: AUDIO_RATE });

  return context;
}

/** Speaks a sound aloud: heard by the core, planned under the tuning as it stands, its first sample heard the lead after the first drawing. */
async function speak(buffer: AudioBuffer): Promise<void> {
  quiet();

  const which = ++playing;
  const output = audio();

  await output.resume().catch(() => undefined);

  if (which !== playing) {
    return;
  }

  heard.value = hear(buffer.getChannelData(0), buffer.sampleRate);
  planned.value = poses(heard.value, props.document.talk.tuning);

  const start = clock.frame() + SETTLE_FRAMES + FACTS.lead;
  const heardIn = (clock.timeOf(start) - performance.now()) / 1000;
  const latency = output.outputLatency || output.baseLatency || 0;

  tell((scene) => scene.speak({ heard: heard.value }, props.face, start));
  source = output.createBufferSource();
  source.buffer = buffer;
  source.connect(output.destination);
  source.onended = () => finish(which);
  source.start(output.currentTime + Math.max(0, heardIn - latency));
  speaking.value = true;
  follow(start);
}

/** Marks on the strip the frame of the sound the face shows, frame by frame, while it plays. */
function follow(start: number): void {
  const step = () => {
    const frame = clock.frame() - start + FACTS.lead;

    shownFrame.value = frame >= 0 && frame < heard.value.length ? frame : null;
    following = requestAnimationFrame(step);
  };

  step();
}

function finish(which: number): void {
  if (which !== playing || !speaking.value) {
    return;
  }

  try {
    source?.stop();
  } catch {
    // It had already ended.
  }

  source = null;
  speaking.value = false;
  shownFrame.value = null;

  if (following !== null) {
    cancelAnimationFrame(following);
    following = null;
  }

  tell((scene, frame) => scene.stop(frame));
}

function quiet(): void {
  finish(playing);
}

function speakSample(): void {
  const samples = sampleVoice(AUDIO_RATE);
  const buffer = audio().createBuffer(1, samples.length, AUDIO_RATE);

  buffer.getChannelData(0).set(samples);
  fileFailed.value = false;
  void speak(buffer);
}

function pickFile(): void {
  file.value?.click();
}

async function speakFile(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement;
  const picked = input.files?.[0];

  input.value = "";

  if (picked === undefined) {
    return;
  }

  const buffer = await audio()
    .decodeAudioData(await picked.arrayBuffer())
    .catch(() => null);

  fileFailed.value = buffer === null;

  if (buffer !== null) {
    await speak(buffer);
  }
}

watch(
  () => props.face,
  (face) => tell((scene) => scene.pin(face)),
  { immediate: true },
);
watch(blinking, (on) => tell((scene) => scene.setBlinking(on)));
watch(gaze, ([x, y]) => tell((scene, frame) => scene.gaze(x, y, frame)));
onBeforeUnmount(() => {
  quiet();
  void context?.close();
});
</script>

<style scoped>
.preview-body {
  aspect-ratio: 116 / 91;
  padding: 8px;
}
</style>
