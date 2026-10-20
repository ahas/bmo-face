import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";

import { chromium } from "playwright-core";
import tailwind from "tailwindcss/colors";

import type { Browser, Page } from "playwright-core";

// What the screens' shared theme promises, measured in a browser: every badge's words read at
// 4.5:1 on every surface a badge rests on, and the one focus ring reads at 3:1 on the same
// surfaces, for every colour, every badge shape and both themes (main.css, 「The one focus ring」;
// docs/face.md, 「The screens' shared style」). The browser paints the tokens the theme states and
// the measuring routine reads what each element ended up as, rather than the numbers written in the
// stylesheet.

(globalThis as { defineAppConfig?: unknown }).defineAppConfig = (config: unknown) => config;

const config = (await import("../../app/app.config")).default as { ui: { colors: Record<string, string> } };
const css = await Bun.file(new URL("../../app/assets/css/main.css", import.meta.url)).text();

// The tones the library's default theme leaves alone, and the app's own two.
const COLORS = { secondary: "blue", success: "green", info: "blue", warning: "yellow", error: "red", ...config.ui.colors };
const TONES = Object.keys(COLORS);
const NEUTRAL = "neutral";
const SHADES = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950] as const;
const SHAPES = ["solid", "outline", "soft", "subtle"] as const;
const SURFACES = ["page", "muted", "elevated"] as const;
const THEMES = ["light", "dark"] as const;

type Shape = (typeof SHAPES)[number];
type Surface = (typeof SURFACES)[number];
type Theme = (typeof THEMES)[number];

// The palette and the theme's tokens, as the library's own stylesheet states them: a scale for every
// colour, the accent step each theme fills with, and the text, background and border tokens.
function tokens(): string {
  const scale = Object.entries(COLORS)
    .flatMap(([tone, name]) => SHADES.map((shade) => `--ui-color-${tone}-${shade}: ${(tailwind as unknown as Record<string, Record<number, string>>)[name]![shade]};`))
    .join("\n");

  const accents = (shade: number) =>
    TONES.filter((tone) => tone !== NEUTRAL)
      .map((tone) => `--ui-${tone}: var(--ui-color-${tone}-${shade});`)
      .join("\n");

  return `:root { ${scale} }
    :root {
      ${accents(500)}
      --ui-text-dimmed: var(--ui-color-neutral-400);
      --ui-text-muted: var(--ui-color-neutral-500);
      --ui-text-toned: var(--ui-color-neutral-600);
      --ui-text: var(--ui-color-neutral-700);
      --ui-text-highlighted: var(--ui-color-neutral-900);
      --ui-text-inverted: #fff;
      --ui-bg: #fff;
      --ui-bg-muted: var(--ui-color-neutral-50);
      --ui-bg-elevated: var(--ui-color-neutral-100);
      --ui-bg-accented: var(--ui-color-neutral-200);
      --ui-bg-inverted: var(--ui-color-neutral-900);
      --ui-border: var(--ui-color-neutral-200);
      --ui-border-muted: var(--ui-color-neutral-200);
      --ui-border-accented: var(--ui-color-neutral-300);
      --ui-border-inverted: var(--ui-color-neutral-900);
    }
    .dark {
      ${accents(400)}
      --ui-text-dimmed: var(--ui-color-neutral-500);
      --ui-text-muted: var(--ui-color-neutral-400);
      --ui-text-toned: var(--ui-color-neutral-300);
      --ui-text: var(--ui-color-neutral-200);
      --ui-text-highlighted: #fff;
      --ui-text-inverted: var(--ui-color-neutral-900);
      --ui-bg: var(--ui-color-neutral-900);
      --ui-bg-muted: var(--ui-color-neutral-800);
      --ui-bg-elevated: var(--ui-color-neutral-800);
      --ui-bg-accented: var(--ui-color-neutral-700);
      --ui-bg-inverted: #fff;
      --ui-border: var(--ui-color-neutral-800);
      --ui-border-muted: var(--ui-color-neutral-700);
      --ui-border-accented: var(--ui-color-neutral-700);
      --ui-border-inverted: #fff;
    }`;
}

// One badge's words and ground, as the library draws that shape: a solid fill, an outlined word, or
// the word on the colour's own wash. Every badge carries `badge-ink`, which is what darkens a light
// theme's colours to the step its words read at.
function badge(tone: string, shape: Shape, surface: Surface): string {
  const name = tone === NEUTRAL ? "neutral" : tone;
  const wash = `color-mix(in oklab, var(--ui-${tone}) 10%, transparent)`;
  const edge = (percent: number) => `box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--ui-${tone}) ${percent}%, transparent)`;

  const painted: Record<Shape, string> = tone === NEUTRAL
    ? {
        solid: "color: var(--ui-text-inverted); background: var(--ui-bg-inverted)",
        outline: "color: var(--ui-text); box-shadow: inset 0 0 0 1px var(--ui-border-accented)",
        soft: "color: var(--ui-text); background: var(--ui-bg-elevated)",
        subtle: "color: var(--ui-text); background: var(--ui-bg-elevated); box-shadow: inset 0 0 0 1px var(--ui-border-accented)"
      }
    : {
        solid: `color: var(--ui-text-inverted); background: var(--ui-${tone})`,
        outline: `color: var(--ui-${tone}); ${edge(50)}`,
        soft: `color: var(--ui-${tone}); background: ${wash}`,
        subtle: `color: var(--ui-${tone}); background: ${wash}; ${edge(25)}`
      };

  return `<span class="badge-ink" data-read="${surface}/${name}/${shape}" style="${painted[shape]}">1,631</span>`;
}

function fixture(): string {
  const surfaces = SURFACES.map((surface) => {
    const ground = { page: "--ui-bg", muted: "--ui-bg-muted", elevated: "--ui-bg-elevated" }[surface];
    const badges = TONES.flatMap((tone) => SHAPES.map((shape) => badge(tone, shape, surface))).join("");
    const focus = `<span data-read="${surface}/focus" style="color: var(--bmo-focus-color)">1,631</span>`;

    return `<div data-surface="${surface}" style="background: var(${ground}); padding: 8px">${focus}${badges}</div>`;
  }).join("");

  return `<style>${tokens()}</style><style>${css}</style><style>
    /* What the library writes into the utilities layer, so the one ring above it is measured
       against the ring it replaces rather than against nothing. */
    @layer utilities {
      .focus-visible\\:outline-3:focus-visible { outline-style: solid; outline-width: 3px; outline-color: color-mix(in srgb, var(--ui-primary) 25%, transparent); }
      .ring { box-shadow: var(--tw-ring-inset,) 0 0 0 1px var(--tw-ring-color); }
      .ring-inset { --tw-ring-inset: inset; }
      .focus-visible\\:ring-primary:focus-visible { --tw-ring-color: var(--ui-primary); }
    }
    body { margin: 0; font: 400 14px sans-serif; background: var(--ui-bg); }
    [data-read] { display: inline-block; margin: 2px; padding: 2px; border-radius: 4px; }
    #field { display: block; margin: 8px; }
    #mark { margin: 8px; }
  </style>
    ${surfaces}
    <input id="field" class="ring ring-inset focus-visible:ring-primary" value="1,631">
    <button id="mark" type="button" class="focus-visible:outline-3">1,631</button>`;
}

// Whether the browser this driver pins is on the machine; where it is not, the file is skipped.
function ready(): boolean {
  try {
    return existsSync(chromium.executablePath());
  } catch {
    return false;
  }
}

const BROWSER_HOOK_MS = 60_000;
const browserReady = ready();
let browser: Browser | null = null;
let page: Page;

beforeAll(async () => {
  if (!browserReady) {
    return;
  }

  browser = await chromium.launch();
  page = await browser.newPage({ viewport: { width: 1280, height: 1200 } });

  await page.setContent(fixture());
}, BROWSER_HOOK_MS);

afterAll(async () => {
  await browser?.close();
}, BROWSER_HOOK_MS);

// Every `[data-read]`'s ratio against the surface it is painted on, as the browser laid it out. A
// colour is painted onto a canvas of one pixel and read back, so a token written as a colour
// function or a wash of one is composited the way the screen composites it.
async function ratios(): Promise<Record<string, number>> {
  return page.evaluate(() => {
    const pad = document.createElement("canvas").getContext("2d", { willReadFrequently: true })!;
    const colour = (value: string): [number, number, number, number] => {
      pad.fillStyle = "#000000";
      pad.fillStyle = value;

      if (pad.fillStyle === "#000000") {
        pad.fillStyle = "#ffffff";
        pad.fillStyle = value;
      }

      pad.clearRect(0, 0, 1, 1);
      pad.fillRect(0, 0, 1, 1);

      return [...pad.getImageData(0, 0, 1, 1).data].slice(0, 4) as [number, number, number, number];
    };
    const over = (top: [number, number, number, number], bottom: [number, number, number, number]): [number, number, number, number] => {
      const a = top[3] / 255;

      return [top[0] * a + bottom[0] * (1 - a), top[1] * a + bottom[1] * (1 - a), top[2] * a + bottom[2] * (1 - a), 255];
    };
    const ground = (element: Element): [number, number, number, number] => {
      const layers: [number, number, number, number][] = [];

      for (let above: Element | null = element; above !== null; above = above.parentElement) {
        const painted = colour(getComputedStyle(above).backgroundColor);

        if (painted[3] === 0) {
          continue;
        }

        layers.push(painted);

        if (painted[3] === 255) {
          break;
        }
      }

      let surface: [number, number, number, number] = layers.at(-1)?.[3] === 255 ? layers.pop()! : [255, 255, 255, 255];

      for (const layer of layers.reverse()) {
        surface = over(layer, surface);
      }

      return surface;
    };
    const luminance = ([r, g, b]: [number, number, number, number]): number => {
      const linear = [r, g, b].map((value) => {
        const part = value / 255;

        return part <= 0.03928 ? part / 12.92 : ((part + 0.055) / 1.055) ** 2.4;
      });

      return 0.2126 * linear[0]! + 0.7152 * linear[1]! + 0.0722 * linear[2]!;
    };

    return Object.fromEntries([...document.querySelectorAll("[data-read]")].map((element) => {
      const style = getComputedStyle(element);
      const back = ground(element);
      const front = over(colour(style.color), back);
      const bright = Math.max(luminance(front), luminance(back));
      const dark = Math.min(luminance(front), luminance(back));

      return [element.getAttribute("data-read")!, Math.round(((bright + 0.05) / (dark + 0.05)) * 100) / 100];
    }));
  });
}

async function themed(theme: Theme): Promise<Record<string, number>> {
  await page.evaluate((dark) => document.documentElement.classList.toggle("dark", dark), theme === "dark");

  return ratios();
}

function below(found: Record<string, number>, floor: number): [string, number][] {
  return Object.entries(found).filter(([, ratio]) => ratio < floor);
}

describe.skipIf(!browserReady)("the shared theme's contrasts", () => {
  for (const theme of THEMES) {
    test(`reads every badge's words at 4.5:1 on every surface in the ${theme} theme`, async () => {
      const found = await themed(theme);
      const words = Object.fromEntries(Object.entries(found).filter(([name]) => name.split("/").length === 3));

      expect(Object.keys(words)).toHaveLength(SURFACES.length * TONES.length * SHAPES.length);
      expect(below(words, 4.5)).toEqual([]);
    });

    test(`reads the focus ring at 3:1 on every surface in the ${theme} theme`, async () => {
      const found = await themed(theme);
      const ring = Object.fromEntries(Object.entries(found).filter(([name]) => name.endsWith("/focus")));

      expect(Object.keys(ring)).toHaveLength(SURFACES.length);
      expect(below(ring, 3)).toEqual([]);
    });
  }
});

// A press on the page before Tab, so the keyboard's walk starts from the top rather than from
// whatever the test before it left focused.
async function keyboardFromTop(): Promise<void> {
  await page.evaluate(() => document.documentElement.classList.remove("dark"));
  await page.mouse.click(0, 0);
}

describe.skipIf(!browserReady)("the one focus ring", () => {
  test("is 2px of the accent, outside the element, for the keyboard", async () => {
    await keyboardFromTop();
    await page.keyboard.press("Tab");
    await page.keyboard.press("Tab");

    const ring = await page.evaluate(() => {
      const marker = document.createElement("span");
      marker.style.color = "var(--bmo-focus-color)";
      document.body.append(marker);

      const style = getComputedStyle(document.querySelector("#mark")!);
      const token = getComputedStyle(marker).color;

      marker.remove();

      return { width: style.outlineWidth, style: style.outlineStyle, offset: style.outlineOffset, colour: style.outlineColor, token };
    });

    expect(ring.width).toBe("2px");
    expect(ring.style).toBe("solid");
    expect(ring.offset).toBe("2px");
    expect(ring.colour).toBe(ring.token);
  });

  test("is not drawn for a press", async () => {
    await page.mouse.click(0, 0);
    await page.locator("#mark").click();

    expect(await page.evaluate(() => getComputedStyle(document.querySelector("#mark")!).outlineStyle)).toBe("none");
  });

  test("leaves a field to show focus on its own edge", async () => {
    await keyboardFromTop();
    await page.keyboard.press("Tab");

    const field = await page.evaluate(() => {
      const style = getComputedStyle(document.querySelector("#field")!);

      return {
        outline: style.outlineStyle,
        edge: style.getPropertyValue("--tw-ring-color").trim(),
        token: getComputedStyle(document.documentElement).getPropertyValue("--bmo-focus-color").trim()
      };
    });

    expect(field.outline).toBe("none");
    expect(field.token).not.toBe("");
    expect(field.edge).toBe(field.token);
  });
});
