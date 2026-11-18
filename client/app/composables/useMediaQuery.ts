import type { Ref } from "vue";

// Whether the screen answers a media query. The page is generated before any browser reads it, so the
// first frame answers false and the query is read once the page is taken over, so the frame the
// browser draws first is the one it was handed.
export function useMediaQuery(query: string): Ref<boolean> {
  const matches = ref(false);

  if (import.meta.client) {
    const media = window.matchMedia(query);
    const read = (): void => {
      matches.value = media.matches;
    };

    onMounted(() => {
      read();
      media.addEventListener("change", read);
    });

    onScopeDispose(() => media.removeEventListener("change", read));
  }

  return matches;
}
