import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { chromium } from "playwright-core";

import { FACE_NAMES, FALLBACK_FACE } from "#faces";

import { BROWSER_HOOK_MS, browserReady } from "../support/browser";
import { CLIENT, run, startServer } from "../support/server";

import type { Browser, Page } from "playwright-core";

import type { FaceServer } from "../support/server";

// The editor's select as the generated screen draws it in a real browser, on the installed select
// menu and its patches (docs/face.md, 「The screens' shared style」): the face list, long enough to
// search, opens with its search holding the focus, narrows to the face its words name with the words
// marked, and the control names the face chosen from it. Which choices words keep is the choices
// module's own test; this file holds what only the page shows. Skipped where the browser build is not
// on the machine.

const LIST = "[data-slot=content]";
const FIELD = `${LIST} input`;
const CHOICE = `${LIST} [data-slot=item]`;
const SETUP_MS = 600_000;
const EDITOR = { name: "tester", password: "test-password" };

interface Words {
  face: {
    name: Record<string, string>;
    signin: Record<string, string>;
    edit: { target: Record<string, string> };
  };
}

const words = Bun.YAML.parse(await Bun.file(join(CLIENT, "i18n/locales/ko.yaml")).text()) as Words;
const ready = browserReady();
const scratch = mkdtempSync(join(tmpdir(), "bmo-face-edit-select-"));

let server: FaceServer | null = null;
let browser: Browser | null = null;
let page: Page;

beforeAll(async () => {
  if (!ready) return;

  // The screen, generated into the scratch folder as a deployment generates it into the release.
  run(["bun", "x", "nuxi", "generate"], CLIENT, { BMO_FACE_OUTPUT_DIR: scratch });
  server = await startServer({ scratch, screen: join(scratch, "public"), accounts: [EDITOR] });
  browser = await chromium.launch();
  page = await browser.newPage({ viewport: { width: 1280, height: 900 } });

  await page.goto(`${server.base}/edit`);
  await page.getByLabel(words.face.signin.name!).fill(EDITOR.name);
  await page.getByLabel(words.face.signin.password!).fill(EDITOR.password);
  await page.getByRole("button", { name: words.face.signin.submit! }).click();
  await control().waitFor();
}, SETUP_MS);

// The face select's control, which the editor draws once the sign-in is taken.
function control() {
  return page.getByRole("button", { name: words.face.edit.target.face!, exact: true });
}

afterAll(async () => {
  await browser?.close();
  await server?.stop();
  rmSync(scratch, { recursive: true, force: true });
}, BROWSER_HOOK_MS);

// The labels of the choices the opened list shows, once it shows the count asked for.
async function shownChoices(count: number): Promise<string[]> {
  await page.waitForFunction(([selector, wanted]) => document.querySelectorAll(selector as string).length === wanted, [CHOICE, count] as const);

  return page.locator(CHOICE).allInnerTexts();
}

describe.skipIf(!ready)("the editor's face select", () => {
  const sought = FACE_NAMES.find((name) => name !== FALLBACK_FACE)!;
  const soughtLabel = words.face.name[sought]!;

  test("opens with every face and its search holding the focus", async () => {
    await control().click();

    expect((await shownChoices(FACE_NAMES.length)).map((label) => label.trim())).toEqual(FACE_NAMES.map((name) => words.face.name[name]!));
    await page.waitForFunction((selector) => document.activeElement?.matches(selector) === true, FIELD);
  });

  test("narrows to the face its words name, the words marked", async () => {
    await page.locator(FIELD).fill(soughtLabel);

    expect((await shownChoices(1)).map((label) => label.trim())).toEqual([soughtLabel]);
    expect(await page.locator(`${CHOICE} mark`).allInnerTexts()).toEqual([soughtLabel]);
  });

  test("names the face chosen from the narrowed list", async () => {
    await page.locator(CHOICE).click();
    await page.locator(LIST).waitFor({ state: "detached" });

    expect((await control().innerText()).trim()).toBe(soughtLabel);
  });
});
