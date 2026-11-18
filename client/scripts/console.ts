// Builds the console bundle (docs/face.md, 「The console screen」) into `.output/console`: the module
// `bundle.js`, the stylesheet `bundle.css`, the words `words.json` and the layer bmo-ui's bundle build
// writes beside them, which the face's server answers to a console. bmo-ui's bundle build leaves the
// libraries a console shares unbundled; the module carries the rest.
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

import tailwind from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import AutoImport from "unplugin-auto-import/vite";
import Components from "unplugin-vue-components/vite";
import { consoleBundle } from "bmo-ui/bundle/console-bundle.js";
import postcss from "postcss";
import { build } from "vite";

const client = resolve(import.meta.dir, "..");
const out = resolve(client, process.env.BMO_FACE_OUTPUT_DIR ?? ".output", "console");
const generatedTheme = join(client, ".nuxt/ui.css");
const consoleTheme = join(client, ".nuxt/console-theme.css");
const nuxtUiComponents = join(client, "node_modules/@nuxt/ui/dist/runtime/components");

// The theme's names the stylesheet is built against: the `@theme` blocks of the stylesheet Nuxt UI
// generates for this app, without its sources and its base, read for reference alone.
function writeConsoleTheme(): void {
  if (!existsSync(generatedTheme)) {
    const prepared = Bun.spawnSync(["bun", "x", "nuxi", "prepare"], { cwd: client, stdout: "inherit", stderr: "inherit" });

    if (prepared.exitCode !== 0) {
      throw new Error("console: nuxi prepare failed, so the theme's names could not be read");
    }
  }

  const blocks: string[] = [];

  postcss.parse(readFileSync(generatedTheme, "utf8")).walkAtRules("theme", (rule) => {
    blocks.push(rule.toString());
  });

  writeFileSync(consoleTheme, `${blocks.join("\n\n")}\n`);
}

// A Nuxt UI component named in a template, as the package's own path of its file.
function nuxtUi(name: string): { name: string; from: string } | undefined {
  const file = name.startsWith("U") ? `${name.slice(1)}.vue` : null;

  return file !== null && existsSync(join(nuxtUiComponents, file)) ? { name: "default", from: `@nuxt/ui/components/${file}` } : undefined;
}

// The words: the bundle's namespace of the pack, under its locale.
const pack = Bun.YAML.parse(readFileSync(join(client, "i18n/locales/ko.yaml"), "utf8")) as Record<string, unknown>;
const manifest = JSON.parse(readFileSync(resolve(client, "../console-manifest.json"), "utf8")) as { name: string };
const words = { ko: { [manifest.name]: pack[manifest.name] } };

writeConsoleTheme();

await build({
  root: client,
  configFile: false,
  logLevel: "warn",
  resolve: {
    alias: [
      { find: "#faces", replacement: resolve(client, "../src/faces.ts") },
      { find: "#core", replacement: resolve(client, "../src/core.ts") },
      // The island, the back gesture's layers, the create button and the select are the console's
      // own, so the bundle takes the layer's where the standalone screens have their copies.
      { find: /^~\/(domain\/(?:island|layers)|components\/(?:CreateButton|ChoiceSelect)\.vue)$/, replacement: "bmo-ui/app/$1" },
      { find: /^~\//, replacement: `${join(client, "app")}/` },
    ],
  },
  define: {
    "import.meta.client": "true",
    "import.meta.server": "false",
    "import.meta.dev": "false",
  },
  plugins: [
    consoleBundle({ name: manifest.name, words }),
    AutoImport({
      imports: ["vue", { "vue-i18n": ["useI18n"] }, { "nuxt/app": ["useAppConfig", "useHead", "useRuntimeConfig", "useState"] }],
      dirs: [join(client, "app/composables")],
      dts: false,
      vueTemplate: true,
    }),
    Components({
      dirs: [join(client, "app/components")],
      resolvers: [nuxtUi],
      dts: false,
    }),
    vue(),
    tailwind(),
  ],
  build: {
    outDir: out,
    emptyOutDir: true,
    minify: true,
    cssCodeSplit: false,
    lib: {
      entry: join(client, "app/console/bundle.ts"),
      formats: ["es"],
      fileName: () => "bundle.js",
      cssFileName: "bundle",
    },
  },
});

writeFileSync(join(out, "words.json"), `${JSON.stringify(words)}\n`);

console.log(`console: built the bundle into ${out}`);
