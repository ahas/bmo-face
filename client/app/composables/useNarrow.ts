import type { Ref } from "vue";

// How wide a screen has to be before it is not a phone: Tailwind's own sm, the width BMO's console
// changes its number boxes at too.
const NARROW_QUERY = "(max-width: 639px)";

// Whether this screen is a narrow one, read as useMediaQuery reads any query.
export function useNarrow(): Ref<boolean> {
  return useMediaQuery(NARROW_QUERY);
}
