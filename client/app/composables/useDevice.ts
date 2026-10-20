// This screen's sign-in with BMO: the device token BMO gave it, kept in this browser's own storage
// and sent as a bearer token on every request to BMO (docs/face.md, 「How a screen signs in」).

const TOKEN_KEY = "bmo-face.token";

export type SignInResult = { ok: true } | { ok: false; reason: "wrong" | "unreachable" | "refused" };

export function bmoUrl(path: string): string {
  return `${useRuntimeConfig().app.baseURL}bmo${path}`;
}

export function useDevice() {
  const token = useState<string | null>("device-token", () => (import.meta.client ? localStorage.getItem(TOKEN_KEY) : null));

  async function signIn(name: string, password: string, device: string): Promise<SignInResult> {
    let response: Response;

    try {
      response = await fetch(bmoUrl("/api/app/login"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ name, password, device }),
      });
    } catch {
      return { ok: false, reason: "unreachable" };
    }

    if (response.status === 401 || response.status === 400) {
      return { ok: false, reason: "wrong" };
    }

    if (!response.ok) {
      return { ok: false, reason: response.status === 502 ? "unreachable" : "refused" };
    }

    const answer = (await response.json()) as { token?: unknown };

    if (typeof answer.token !== "string") {
      return { ok: false, reason: "refused" };
    }

    localStorage.setItem(TOKEN_KEY, answer.token);
    token.value = answer.token;

    return { ok: true };
  }

  // A token BMO no longer takes — revoked on its account screen — is forgotten, and the screen asks
  // to be signed in again.
  function forget(): void {
    localStorage.removeItem(TOKEN_KEY);
    token.value = null;
  }

  return { token, signIn, forget };
}
