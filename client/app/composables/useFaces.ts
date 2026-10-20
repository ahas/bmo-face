import { keptDocument, shouldReload } from "~/domain/served";

import type { FaceDocument } from "#faces";

// The expression document, read on opening and again on every save this server announces, so an
// open display draws what the editor saved without being reloaded (docs/face.md, 「The expression
// document」). A document this display's own check refuses is not drawn, and a display running an
// older screen than the one served reloads itself (「Showing on any screen」).

export function useFaces() {
  const { baseURL, buildAssetsDir, buildId } = useRuntimeConfig().app;
  const manifest = `${baseURL}${buildAssetsDir.replace(/^\//, "")}builds/latest.json`;
  const document = ref<FaceDocument | null>(null);
  const failed = ref(false);
  let saves: EventSource | null = null;

  async function load(): Promise<void> {
    try {
      document.value = keptDocument(document.value, await $fetch<unknown>(`${baseURL}api/faces`, { cache: "no-store" }));
      failed.value = document.value === null;
    } catch {
      failed.value = document.value === null;
    }
  }

  // The stream opens again after the server restarts, as it does on a deploy that may have brought
  // a screen this one does not know.
  async function checkBuild(): Promise<void> {
    try {
      if (shouldReload(buildId, await $fetch<unknown>(manifest, { cache: "no-store" }))) {
        reloadNuxtApp({ force: true });
      }
    } catch {
      // No manifest to read: keep running this screen.
    }
  }

  onMounted(() => {
    void load();

    // The browser reopens this stream on its own when it drops; the first frame of every opening
    // names the version standing, which catches up a save missed while it was down.
    let opened = false;

    saves = new EventSource(`${baseURL}api/faces/events`);
    saves.addEventListener("open", () => {
      if (opened) {
        void checkBuild();
      }

      opened = true;
    });
    saves.addEventListener("faces", (event) => {
      const savedAt = (JSON.parse((event as MessageEvent<string>).data) as { savedAt?: unknown }).savedAt;

      if (savedAt !== document.value?.savedAt) {
        void load();
      }
    });
  });
  onBeforeUnmount(() => saves?.close());

  return { document, failed, load };
}
