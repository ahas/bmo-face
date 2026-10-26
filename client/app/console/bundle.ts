import { relayAddress } from "bmo-ui/app/domain/relay";

import manifest from "../../../console-manifest.json";
import EditPage from "../pages/edit.vue";

import type { ConsoleBundleModule } from "bmo-ui/shared/console-manifest";
import type { EditorRoad } from "../composables/useEditor";

import "./bundle.css";

// The console bundle's module (docs/face.md, 「The console screen」): the editor's page at the screen
// path, its calls made through the console's relay to the face's console routes, where the console's
// credential and the reader's role stand in for the editor's own login.

const road: EditorRoad = {
  address: (path) => relayAddress(manifest.serviceName, `${manifest.apiPrefix}${path}`),
  session: false,
};

const bundle: ConsoleBundleModule = {
  // The route asks for the console's title bar, so the screen stands under it (docs/face.md, 「The console screen」).
  routes: [{ path: manifest.screenPath, component: EditPage, props: { road }, meta: { consoleTitleBar: true } }],
};

export default bundle;
