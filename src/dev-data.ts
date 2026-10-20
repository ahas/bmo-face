import { copyFileSync, existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve, sep } from "node:path";

import { DEFAULT_FACES } from "./faces";

// The copy of the data root development works against, so nothing a check saves reaches the live
// document (docs/development.md). Only the document is copied; a bench login makes its own session.

/** The expression document's file in a data root. */
export const FACES_FILE = "faces.json";

export const realDataRoot = join(homedir(), ".local/share/bmo-face");
export const devDataRoot = join(homedir(), ".local/share/bmo/data/workbench/bmo-face/data");

export interface DevDataReport {
  /** Where the copy's document came from: the live data root, or the project's default. */
  document: "live" | "default";
}

/** Makes or refreshes the copy at `to` from the data root at `from`. */
export function refreshDevData(from: string, to: string): DevDataReport {
  const source = resolve(from);
  const target = resolve(to);

  if (target === source || target.startsWith(source + sep)) {
    throw new Error(`the copy may not stand inside the data root it copies: ${target}`);
  }

  mkdirSync(target, { recursive: true });

  if (existsSync(join(source, FACES_FILE))) {
    copyFileSync(join(source, FACES_FILE), join(target, FACES_FILE));

    return { document: "live" };
  }

  rmSync(join(target, FACES_FILE), { force: true });
  writeFileSync(join(target, FACES_FILE), `${JSON.stringify(DEFAULT_FACES, null, 2)}\n`);

  return { document: "default" };
}
