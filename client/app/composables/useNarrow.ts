import type { Ref } from "vue";

// How wide a screen has to be before it is not a phone: Tailwind's own sm, the width BMO's console
// changes its number boxes at too.
const NARROW_QUERY = "(max-width: 639px)";

// Whether this screen is a narrow one. The page is generated before any browser reads it, so the
// first frame answers false and the width is read once the page is taken over, so the frame the
// browser draws first is the one it was handed.
export function useNarrow(): Ref<boolean> {
  const narrow = ref(false);

  if (import.meta.client) {
    const media = window.matchMedia(NARROW_QUERY);
    const read = (): void => {
      narrow.value = media.matches;
    };

    onMounted(() => {
      read();
      media.addEventListener("change", read);
    });

    onScopeDispose(() => media.removeEventListener("change", read));
  }

  return narrow;
}
