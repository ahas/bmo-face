import { fileURLToPath } from "node:url";

export default defineNuxtConfig({
  modules: ["@nuxt/ui", "@nuxtjs/i18n"],
  css: ["~/assets/css/main.css"],
  compatibilityDate: "2025-09-01",
  devtools: { enabled: false },
  // The face is a single page generated into static files; its own server hands them out and
  // answers every other path under the prefix with the page (docs/face.md, 「The server」).
  ssr: false,
  // The expression document is stated once, at the repository's root, and the face's core built
  // there; both are read here as they stand.
  alias: {
    "#faces": fileURLToPath(new URL("../src/faces.ts", import.meta.url)),
    "#core": fileURLToPath(new URL("../src/core.ts", import.meta.url))
  },
  app: {
    baseURL: "/bmo-face/",
    head: {
      htmlAttrs: { lang: "ko" },
      meta: [
        {
          name: "viewport",
          content: "width=device-width, initial-scale=1, viewport-fit=cover"
        },
        { name: "theme-color", content: "#58bfae" }
      ]
    }
  },
  // A generated page has no server to ask for an icon, so every icon the face draws is bundled: its own files', and
  // those of the error page it takes from bmo-ui (docs/face.md, 「The screens' shared style」), which the scan reads
  // inside the installed package, so the installed packages are not passed over as a whole.
  icon: {
    provider: "none",
    clientBundle: {
      scan: {
        globInclude: ["app/**/*.{vue,ts}", "node_modules/bmo-ui/app/components/ErrorScreen.vue", "node_modules/bmo-ui/app/domain/error-page.ts"],
        globExclude: ["dist", "build", "coverage", "test", "tests", ".*"]
      }
    }
  },
  nitro: {
    // A deployment generates into a release directory of its own and names it here.
    output: { dir: process.env.BMO_FACE_OUTPUT_DIR ?? ".output" }
  },
  // The development bench passes what the face's server answers through to it; the Host header is
  // kept, since the server matches a write's Origin against it (docs/development.md).
  $development: {
    nitro: {
      devProxy: Object.fromEntries(
        ["api", "bmo"].map((part) => [
          `/bmo-face/${part}`,
          { target: `${process.env.BMO_FACE_DEV_API ?? "http://127.0.0.1:3218"}/bmo-face/${part}`, changeOrigin: false }
        ])
      )
    },
    vite: { server: { allowedHosts: ["dev.example.test"] } }
  },
  // Four packs holding the same keys, one to a locale under `i18n/locales/`: every word the face
  // shows is a key into them. The standalone screens speak Korean; the console bundle carries all four.
  i18n: {
    strategy: "no_prefix",
    defaultLocale: "ko",
    detectBrowserLanguage: false,
    locales: [
      { code: "ko", language: "ko-KR", file: "ko.yaml" },
      { code: "en", language: "en-US", file: "en.yaml" },
      { code: "ja", language: "ja-JP", file: "ja.yaml" },
      { code: "zh-CN", language: "zh-CN", file: "zh-CN.yaml" }
    ]
  },
  ui: {
    theme: {
      colors: ["primary", "secondary", "success", "info", "warning", "error", "neutral"]
    }
  }
});
