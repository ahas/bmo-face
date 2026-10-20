import { fileURLToPath } from "node:url";

export default defineNuxtConfig({
  modules: ["@nuxt/ui", "@nuxtjs/i18n"],
  css: ["~/assets/css/main.css"],
  compatibilityDate: "2025-09-01",
  devtools: { enabled: false },
  // The face is a single page generated into static files; its own server hands them out and
  // answers every other path under the prefix with the page (docs/face.md, 「The server」).
  ssr: false,
  // The expression document is stated once, at the repository's root, and read here as it stands.
  alias: {
    "#faces": fileURLToPath(new URL("../src/faces.ts", import.meta.url))
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
  // A generated page has no server to ask for an icon, so every icon the face draws is bundled.
  icon: {
    provider: "none",
    clientBundle: { scan: { globInclude: ["app/**/*.{vue,ts}"] } }
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
  // One language, one pack: every word the face shows is a key into `i18n/locales/ko.yaml`.
  i18n: {
    strategy: "no_prefix",
    defaultLocale: "ko",
    detectBrowserLanguage: false,
    locales: [{ code: "ko", language: "ko-KR", name: "한국어", file: "ko.yaml" }]
  },
  ui: {
    theme: {
      colors: ["primary", "secondary", "success", "info", "warning", "error", "neutral"]
    }
  }
});
