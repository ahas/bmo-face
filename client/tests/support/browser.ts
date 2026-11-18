import { existsSync } from "node:fs";

import { chromium } from "playwright-core";

// What every browser test of the screen shares: whether the browser the driver pins is on the
// machine, which skips the file where it is not, and the bound a launch or a close is held to.

/** The longest a browser's launch or close may take. */
export const BROWSER_HOOK_MS = 60_000;

/** Whether the browser this driver pins is on the machine. */
export function browserReady(): boolean {
  try {
    return existsSync(chromium.executablePath());
  } catch {
    return false;
  }
}
