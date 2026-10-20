// How much of the window a person can actually see and reach. On a phone a virtual keyboard is laid
// over the page rather than cut out of it, and `innerHeight` stays as it was while the foot of the
// screen has gone behind the keys. The visual viewport is where a browser reports what is left, and
// its offset with it, since a browser may push the page up to keep the focused box in sight.

// A tolerance rather than an equality: the two heights are fractional, and a pixel of difference is
// a rounding rather than something covering the screen.
const VISIBLE_TOLERANCE_PX = 1;

export interface VisibleReading {
  windowHeight: number;
  height: number;
  offsetTop: number;
  scale: number;
}

// Where a full-screen frame stands while something covers the window, and nothing when the whole
// window is visible, so the frame then follows the window and the browser's own bars.
export interface VisibleFrame {
  top: number;
  height: number;
}

export function visibleFrame(reading: VisibleReading | null): VisibleFrame | null {
  // A pinched screen is a magnified one, not a covered one.
  if (reading === null || reading.scale > 1) {
    return null;
  }

  const height = Math.round(reading.height);
  const top = Math.round(reading.offsetTop);
  const whole = height >= reading.windowHeight - VISIBLE_TOLERANCE_PX;

  return whole && top === 0 ? null : { top, height };
}

// The two properties the frame is laid out from; the stylesheet's own defaults are the whole window.
export const SCREEN_HEIGHT_PROPERTY = "--bmo-screen";
export const SCREEN_TOP_PROPERTY = "--bmo-screen-top";

export const SCREEN_FRAME = { top: `var(${SCREEN_TOP_PROPERTY})`, height: `var(${SCREEN_HEIGHT_PROPERTY})` } as const;

// Keeps those two properties saying what is visible for as long as the screen is open.
export function useVisibleScreen(): void {
  if (!import.meta.client) {
    return;
  }

  const read = (): void => {
    const visual = window.visualViewport;
    const frame = visibleFrame(
      visual === null
        ? null
        : { windowHeight: window.innerHeight, height: visual.height, offsetTop: visual.offsetTop, scale: visual.scale }
    );
    const style = document.documentElement.style;

    if (frame === null) {
      style.removeProperty(SCREEN_HEIGHT_PROPERTY);
      style.removeProperty(SCREEN_TOP_PROPERTY);

      return;
    }

    style.setProperty(SCREEN_HEIGHT_PROPERTY, `${frame.height}px`);
    style.setProperty(SCREEN_TOP_PROPERTY, `${frame.top}px`);
  };

  // A keyboard opening resizes the visible part; a browser pushing its page up scrolls it.
  onMounted(() => {
    read();
    window.visualViewport?.addEventListener("resize", read);
    window.visualViewport?.addEventListener("scroll", read);
  });

  onScopeDispose(() => {
    window.visualViewport?.removeEventListener("resize", read);
    window.visualViewport?.removeEventListener("scroll", read);
  });
}
