import { useIsland } from "~/domain/island";
import { changed, refusalOf, saveBody } from "~/domain/edit";

import type { FaceDocument } from "#faces";

// The editor's session and its document (docs/face.md, 「The editor」): who is signed in, the document
// as saved and as being edited, and the save that sends one to become the other.

export function useEditor() {
  const { t } = useI18n();
  const island = useIsland(t);
  const base = useRuntimeConfig().app.baseURL;

  /** Who is signed in; null for nobody, undefined while that is still being asked. */
  const editor = ref<string | null | undefined>(undefined);
  const saved = ref<FaceDocument | null>(null);
  const edited = ref<FaceDocument | null>(null);
  const loadFailed = ref(false);
  const saving = ref(false);
  const refusal = ref<string | null>(null);

  const dirty = computed(() => saved.value !== null && edited.value !== null && changed(edited.value, saved.value));

  async function ask<T>(path: string, init: RequestInit = {}): Promise<{ status: number; body: T | null }> {
    try {
      const response = await fetch(`${base}api${path}`, { cache: "no-store", ...init, headers: { "content-type": "application/json", ...init.headers } });
      const text = await response.text();

      return { status: response.status, body: text === "" ? null : (JSON.parse(text) as T) };
    } catch {
      return { status: 0, body: null };
    }
  }

  async function whoAmI(): Promise<void> {
    const me = await ask<{ name: string }>("/me");

    editor.value = me.status === 200 ? (me.body?.name ?? null) : null;
  }

  async function load(): Promise<void> {
    const answer = await ask<FaceDocument>("/faces");

    if (answer.status !== 200 || answer.body === null) {
      loadFailed.value = saved.value === null;

      return;
    }

    loadFailed.value = false;
    saved.value = answer.body;
    edited.value = structuredClone(answer.body);
    refusal.value = null;
  }

  async function login(name: string, password: string): Promise<"ok" | "wrong" | "failed"> {
    const answer = await ask<{ name: string }>("/login", { method: "POST", body: JSON.stringify({ name, password }) });

    if (answer.status === 200) {
      editor.value = answer.body?.name ?? name;
      await load();

      return "ok";
    }

    return answer.status === 401 || answer.status === 400 ? "wrong" : "failed";
  }

  async function logout(): Promise<void> {
    await ask("/logout", { method: "POST" });
    editor.value = null;
  }

  async function save(): Promise<void> {
    if (edited.value === null || saved.value === null || saving.value) {
      return;
    }

    // Refused here first with the sentence the server would give, without sending anything.
    refusal.value = refusalOf(edited.value);

    if (refusal.value !== null) {
      return;
    }

    saving.value = true;

    const answer = await ask<FaceDocument & { error?: string }>("/faces", {
      method: "PUT",
      body: JSON.stringify(saveBody(edited.value, saved.value.savedAt)),
    });

    saving.value = false;

    if (answer.status === 200 && answer.body !== null) {
      saved.value = answer.body;
      edited.value = { ...edited.value, savedAt: answer.body.savedAt };
      island.say({ key: "edit-saved", tone: "success", title: t("edit.status.saved") });

      return;
    }

    if (answer.status === 409) {
      const reload = await island.ask({ title: t("edit.conflict"), confirm: t("edit.action.reload") });

      if (reload) {
        await load();
      }

      return;
    }

    if (answer.status === 401) {
      editor.value = null;

      return;
    }

    refusal.value = (answer.body as { error?: string } | null)?.error ?? t("edit.error.save_failed");
  }

  onMounted(async () => {
    await whoAmI();

    if (editor.value !== null) {
      await load();
    }
  });

  return { editor, saved, edited, loadFailed, saving, refusal, dirty, login, logout, load, save };
}
