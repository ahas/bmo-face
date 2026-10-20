import { checkFaceDocument } from "#faces";

import type { FaceDocument } from "#faces";

// What the server serves a display, taken only when this display can draw it (docs/face.md,
// 「Showing on any screen」): the expression document, and the build of the screen itself.

/**
 * Whether a display running build `runningId` stands behind the screen the server serves now, as
 * the served build manifest names it. A manifest that names no build reloads nothing.
 */
export function shouldReload(runningId: string, manifest: unknown): boolean {
  if (typeof manifest !== "object" || manifest === null) {
    return false;
  }

  const { id } = manifest as { id?: unknown };

  return typeof id === "string" && id !== "" && id !== runningId;
}

/** The document to draw: what arrived when this display's own check passes it, else the one drawn now. */
export function keptDocument(drawn: FaceDocument | null, arrived: unknown): FaceDocument | null {
  const checked = checkFaceDocument(arrived);
  const savedAt = (arrived as { savedAt?: unknown } | null)?.savedAt;

  if ("error" in checked || typeof savedAt !== "number") {
    return drawn;
  }

  return { ...checked.ok, savedAt };
}
