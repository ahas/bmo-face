<template>
  <SignIn v-if="token === null" />

  <main
    v-else
    class="face-page"
    :style="{ background: document?.colors.body ?? DEFAULT_FACES.colors.body }"
    :data-face="shown.face"
    :data-stage="shown.talking ? stage : ''"
    @pointerdown="touched"
  >
    <FaceView
      v-if="document"
      :colors="document.colors"
      :face="faceFor(document, shown.face)"
      :mouth="shown.talking ? (document.mouths[stage] ?? null) : null"
      :label="t(`face.name.${shown.face}`)"
    />

    <p v-else-if="failed" class="face-note">{{ t("face.unloaded") }}</p>

    <button v-if="suspended" type="button" class="face-note face-start" @click="touched">
      {{ t("face.start") }}
    </button>

    <div v-if="speaking" class="face-stop">
      <UButton color="neutral" variant="solid" size="xl" icon="i-material-symbols-stop-outline" :label="t('face.stop')" @click.stop="stopListening" />
    </div>
  </main>
</template>

<script setup lang="ts">
import { DEFAULT_FACES, faceFor } from "#faces";

import { nextChange, shownFace } from "~/domain/expression";
import { briefingKey, isFresh } from "~/domain/listener";

import type { FaceState } from "#faces";
import type { FaceMoment } from "~/domain/expression";
import type { Heard } from "~/domain/listener";

// The display (docs/face.md): BMO's face, full screen, moved by what BMO's listener stream says.

const { t } = useI18n();
const route = useRoute();
const { token, forget } = useDevice();
const { document, failed } = useFaces();

const silent = route.query.sound === "off";
const stages = computed(() => document.value?.mouths.length ?? 1);
const { speaking, spoke, stage, suspended, play, stop, resume } = useSpeaker({ token, silent, stages });

const presence = ref<{ state: FaceState; since: number } | null>(null);
const now = ref(Date.now());
const heardBefore = new Set<string>();

function heard(news: Heard): void {
  if (news.kind === "presence") {
    // The since is the console's clock; one ahead of this screen's is read as now.
    presence.value = { state: news.state, since: Math.min(news.since, Date.now()) };
  } else if (news.kind === "briefing") {
    const key = briefingKey(news.briefing);

    if (!heardBefore.has(key) && isFresh(news.briefing.sentAt, Date.now())) {
      heardBefore.add(key);
      void play(news.briefing);
    }
  } else if (news.kind === "stopped" && speaking.value !== null && briefingKey(speaking.value) === briefingKey(news)) {
    stop();
  }
}

useListener(token, { heard, refused: forget });

const moment = computed<FaceMoment>(() => ({
  speaking: speaking.value === null ? null : { emotion: speaking.value.emotion },
  spoke: spoke.value,
  presence: presence.value,
  now: now.value,
}));
const shown = computed(() => shownFace(moment.value));

// The face changes on its own when an emotion's linger ends and when an idle turns to sleep; the
// clock is looked at again at the next of those, and whenever something new is heard.
let tick: ReturnType<typeof setTimeout> | null = null;

watchEffect(() => {
  if (tick !== null) {
    clearTimeout(tick);
  }

  const next = nextChange(moment.value);

  if (next !== null) {
    tick = setTimeout(() => (now.value = Date.now()), Math.max(0, next - Date.now()) + 50);
  }
});
watch([speaking, spoke, presence], () => (now.value = Date.now()));

// 그만듣기: stops the sound here and tells every other listener to stop too.
async function stopListening(): Promise<void> {
  const playing = speaking.value;

  stop();

  if (playing !== null && token.value !== null) {
    await fetch(bmoUrl("/api/briefing/stop"), {
      method: "POST",
      headers: { authorization: `Bearer ${token.value}`, "content-type": "application/json" },
      body: JSON.stringify({ chatId: playing.chatId, messageId: playing.messageId }),
    }).catch(() => undefined);
  }
}

// The first touch asks for the whole screen and lets the sound out; a browser allows neither before
// one.
function touched(): void {
  if (!globalThis.document.fullscreenElement) {
    void globalThis.document.documentElement.requestFullscreen?.().catch(() => undefined);
  }

  void resume();
}

// The screen is kept awake where the browser can, and asked again whenever the page comes back.
let lock: WakeLockSentinel | null = null;

async function keepAwake(): Promise<void> {
  if (globalThis.document.visibilityState === "visible" && "wakeLock" in navigator) {
    lock = await navigator.wakeLock.request("screen").catch(() => null);
  }
}

onMounted(() => {
  void keepAwake();
  globalThis.document.addEventListener("visibilitychange", keepAwake);
});
onBeforeUnmount(() => {
  globalThis.document.removeEventListener("visibilitychange", keepAwake);
  void lock?.release();

  if (tick !== null) {
    clearTimeout(tick);
  }
});

useHead({ title: () => t("face.title") });
</script>

<style scoped>
.face-page {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: max(2vmin, env(safe-area-inset-top, 0px)) max(2vmin, env(safe-area-inset-right, 0px)) max(2vmin, env(safe-area-inset-bottom, 0px))
    max(2vmin, env(safe-area-inset-left, 0px));
  overflow: hidden;
  touch-action: manipulation;
  user-select: none;
  transition: background-color 300ms ease-out;
}

.face-note {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #0b2a25;
  font-size: 1.5rem;
  font-weight: 600;
}

.face-start {
  background: rgb(0 0 0 / 0.35);
  color: #ffffff;
  cursor: pointer;
}

.face-start:hover {
  background: rgb(0 0 0 / 0.45);
}

.face-start:active {
  background: rgb(0 0 0 / 0.55);
}

.face-stop {
  position: absolute;
  right: max(16px, env(safe-area-inset-right, 0px));
  bottom: max(16px, env(safe-area-inset-bottom, 0px));
}
</style>
