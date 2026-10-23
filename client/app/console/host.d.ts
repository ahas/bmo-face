// What a console's app configuration holds that the bundle reads through bmo-ui's `relay` module
// (bmo-ui's `docs/console-bundles.md`, "What a console owes"), declared for the screen's own
// typecheck: the standalone screens hold none of it, and in a console the console's own stands.
declare module "nuxt/schema" {
  interface AppConfig {
    bmoUi: {
      relayBase?: string;
    };
  }
}

export {};
