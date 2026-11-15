import { useIsland } from "~/domain/island";
import { changed, refusalOf, refusalWords, saveBody } from "~/domain/edit";

import type { FaceDocument, Refusal } from "#faces";

// The editor's session and its document (docs/face.md, 「The editor」): who is signed in, the document
// as saved and as being edited, and the save that sends one to become the other.

/** Where the editor's calls go, and whether it signs in there. */
export interface EditorRoad {
  /** The address of one of the editor's calls, named as the server's own API names it: `/faces`. */
  address: (path: string) => string;
  /** Whether the editor signs in on this road; false where whoever opens it is already signed in. */
  session: boolean;
}

/** The face server's own routes under the page's base, behind the editor's own login. */
export function sessionRoad(): EditorRoad {
  const base = useRuntimeConfig().app.baseURL;

  return { address: (path) => `${base}api${path}`, session: true };
}

export function useEditor(road: EditorRoad) {
  const { t } = useI18n();
  const island = useIsland(t);

  /** Who is signed in; null for nobody, undefined while that is still being asked, and empty on a road
   * with no session of the editor's own. */
  const editor = ref<string | null | undefined>(undefined);
  const saved = ref<FaceDocument | null>(null);
  const edited = ref<FaceDocument | null>(null);
  const loadFailed = ref(false);
  const saving = ref(false);
  const refusal = ref<string | null>(null);

  const dirty = computed(() => saved.value !== null && edited.value !== null && changed(edited.value, saved.value));

  async function ask<T>(path: string, init: RequestInit = {}): Promise<{ status: number; body: T | null }> {
    try {
      const response = await fetch(road.address(path), { cache: "no-store", ...init, headers: { "content-type": "application/json", ...init.headers } });
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

    // Refused here first by the same check the server holds, without sending anything.
    const refused = refusalOf(edited.value);

    if (refused !== null) {
      refusal.value = refusalWords(refused, t);

      return;
    }

    refusal.value = null;

    saving.value = true;

    const answer = await ask<FaceDocument & { refusal?: Refusal }>("/faces", {
      method: "PUT",
      body: JSON.stringify(saveBody(edited.value, saved.value.savedAt)),
    });

    saving.value = false;

    if (answer.status === 200 && answer.body !== null) {
      saved.value = answer.body;
      edited.value = { ...edited.value, savedAt: answer.body.savedAt };
      island.say({ key: "edit-saved", tone: "success", title: t("face.edit.status.saved") });

      return;
    }

    if (answer.status === 409) {
      const reload = await island.ask({ title: t("face.edit.conflict"), confirm: t("face.edit.action.reload") });

      if (reload) {
        await load();
      }

      return;
    }

    if (answer.status === 401 && road.session) {
      editor.value = null;

      return;
    }

    // A road with no session of the editor's own is refused by whoever signed the reader in, in
    // words that are not the screen's.
    if ((answer.status === 401 || answer.status === 403) && !road.session) {
      refusal.value = t("face.edit.error.save_failed");

      return;
    }

    const said = (answer.body as { refusal?: Refusal } | null)?.refusal;

    refusal.value = said === undefined ? t("face.edit.error.save_failed") : refusalWords(said, t);
  }

  onMounted(async () => {
    if (!road.session) {
      editor.value = "";
      await load();

      return;
    }

    await whoAmI();

    if (editor.value !== null) {
      await load();
    }
  });

  return { editor, saved, edited, loadFailed, saving, refusal, dirty, login, logout, load, save };
}
