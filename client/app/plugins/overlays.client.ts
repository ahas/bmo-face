import { watchOverlays } from "~/domain/overlays";

// What floats over the page closes to the back gesture and to a finger's tap away from it, on
// every screen, from the moment the app is up.
export default defineNuxtPlugin(() => {
  watchOverlays();
});
