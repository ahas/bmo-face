// Builds the console bundle (docs/face.md, 「The console screen」) into `.output/console`: the module
// `bundle.js`, the stylesheet `bundle.css` and the words `words.json`, which the face's server answers
// to a console. The module leaves the libraries a console shares unbundled and carries the rest.
import { existsSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

import tailwind from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import AutoImport from "unplugin-auto-import/vite";
import Components from "unplugin-vue-components/vite";
import { build } from "vite";

const client = resolve(import.meta.dir, "..");
const out = resolve(client, process.env.BMO_FACE_OUTPUT_DIR ?? ".output", "console");
const generatedTheme = join(client, ".nuxt/ui.css");
const consoleTheme = join(client, ".nuxt/console-theme.css");
const nuxtUiComponents = join(client, "node_modules/@nuxt/ui/dist/runtime/components");

// What a console shares with every bundle, left as the package's own path for the console to resolve.
const SHARED = /^(?:vue|vue-router|vue-i18n|nuxt\/app|@nuxt\/ui|bmo-ui)(?:\/|$)/;

// The theme's names the stylesheet is built against: the `@theme` blocks of the stylesheet Nuxt UI
// generates for this app, without its sources and its base, read for reference alone.
function writeConsoleTheme(): void {
  if (!existsSync(generatedTheme)) {
    const prepared = Bun.spawnSync(["bun", "x", "nuxi", "prepare"], { cwd: client, stdout: "inherit", stderr: "inherit" });

    if (prepared.exitCode !== 0) {
      throw new Error("console: nuxi prepare failed, so the theme's names could not be read");
    }
  }

  const blocks = readFileSync(generatedTheme, "utf8").match(/@theme[^{]*\{[^}]*\}/g) ?? [];

  writeFileSync(consoleTheme, `${blocks.join("\n\n")}\n`);
}

// A Nuxt UI component named in a template, as the package's own path of its file.
function nuxtUi(name: string): { name: string; from: string } | undefined {
  const file = name.startsWith("U") ? `${name.slice(1)}.vue` : null;

  return file !== null && existsSync(join(nuxtUiComponents, file)) ? { name: "default", from: `@nuxt/ui/components/${file}` } : undefined;
}

writeConsoleTheme();

await build({
  root: client,
  configFile: false,
  logLevel: "warn",
  resolve: {
    alias: [
      { find: "#faces", replacement: resolve(client, "../src/faces.ts") },
      // The island and the back gesture's layers are the console's own, so the bundle takes the
      // layer's modules where the standalone screens have their copies.
      { find: /^~\/domain\/(island|layers)$/, replacement: "bmo-ui/app/domain/$1" },
      { find: /^~\//, replacement: `${join(client, "app")}/` },
    ],
  },
  define: {
    "import.meta.client": "true",
    "import.meta.server": "false",
    "import.meta.dev": "false",
  },
  plugins: [
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
    rollupOptions: {
      external: (id) => SHARED.test(id),
    },
  },
});

// The words: the bundle's namespace of the pack, under its locale.
const pack = Bun.YAML.parse(readFileSync(join(client, "i18n/locales/ko.yaml"), "utf8")) as Record<string, unknown>;
const manifest = JSON.parse(readFileSync(resolve(client, "../console-manifest.json"), "utf8")) as { name: string };

writeFileSync(join(out, "words.json"), `${JSON.stringify({ ko: { [manifest.name]: pack[manifest.name] } })}\n`);

for (const file of readdirSync(out)) {
  if (!["bundle.js", "bundle.css", "words.json"].includes(file)) {
    rmSync(join(out, file), { recursive: true });
  }
}

console.log(`console: built the bundle into ${out}`);
