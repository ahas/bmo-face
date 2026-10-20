// Text cut short to fit its box, found where the reader is: the line under the pointer, the line
// the keyboard has focused, or the line under a held finger. The box may be the line's own, as a
// single line that ends in an ellipsis or a block clamped to a few, or a parent that hides what runs
// past its edge. Whether it is cut is measured as the tooltip is about to open, so the answer is the
// width of the moment rather than the one the line had when it was drawn.
//
// A line that already has a tooltip of its own is left to it, so the marks a tooltip writes on what
// it stands on are where the search stops.

export interface Rect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface Cut {
  /** Where the text runs, whether or not it can be seen. */
  text: Rect;
  /** The edge it is cut at. */
  box: Rect;
}

export interface Clipped {
  /** The whole of the cut line, as the reader would read it if the box were wide enough. */
  text: string;
  /** The part of the line that can be seen. */
  rect: Rect;
}

// What a tooltip leaves on what it stands on, and on what it draws.
const TOOLTIP_MARKS = "[data-bmo-tooltip], [data-bmo-clipped]";

// One device pixel of slack: text boxes land on fractions, and a line that meets its box exactly
// is not cut.
const SLACK = 1;

const CLIPPING = new Set(["hidden", "clip"]);

// Whether the line runs past the edge it is cut at.
export function isCut(cut: Cut, slack = SLACK): boolean {
  return (
    cut.text.right > cut.box.right + slack ||
    cut.text.left < cut.box.left - slack ||
    cut.text.bottom > cut.box.bottom + slack ||
    cut.text.top < cut.box.top - slack
  );
}

// The part of the line that stands inside the box, or null when nothing does.
export function visiblePart(cut: Cut): Rect | null {
  const rect = {
    left: Math.max(cut.text.left, cut.box.left),
    top: Math.max(cut.text.top, cut.box.top),
    right: Math.min(cut.text.right, cut.box.right),
    bottom: Math.min(cut.text.bottom, cut.box.bottom),
  };

  return rect.right > rect.left && rect.bottom > rect.top ? rect : null;
}

// The nearest line the element belongs to: the element itself when it holds text of its own, and
// otherwise the first of its parents that does.
export function lineOf(element: Element): Element | null {
  for (let at: Element | null = element; at !== null; at = at.parentElement) {
    if (holdsText(at)) {
      return at;
    }
  }

  return null;
}

// The line at a point on the page, read through the text itself: a line may let the pointer
// through to what stands behind it, and what the pointer meets there is not the line.
export function lineAt(x: number, y: number): Element | null {
  const caret = document as Document & {
    caretRangeFromPoint?: (x: number, y: number) => Range | null;
    caretPositionFromPoint?: (x: number, y: number) => { offsetNode: Node } | null;
  };
  const range = caret.caretRangeFromPoint?.(x, y) ?? null;
  const node = range?.startContainer ?? caret.caretPositionFromPoint?.(x, y)?.offsetNode ?? null;

  if (node === null || node.nodeType !== Node.TEXT_NODE) {
    return null;
  }

  const element = node.parentElement;

  return element === null ? null : lineOf(element);
}

// The cut line at a point on the page, if there is one.
export function clippedAt(x: number, y: number): Clipped | null {
  const line = lineAt(x, y);

  return line === null || line.closest(TOOLTIP_MARKS) !== null ? null : cutLine(line);
}

// The cut line the element stands in, if it stands in one.
export function clippedOf(element: Element): Clipped | null {
  if (element.closest(TOOLTIP_MARKS) !== null) {
    return null;
  }

  const line = lineOf(element);

  return line === null ? null : cutLine(line);
}

// The first cut line inside something the keyboard has focused.
export function clippedWithin(root: Element): Clipped | null {
  if (root.closest(TOOLTIP_MARKS) !== null) {
    return null;
  }

  for (const line of [root, ...Array.from(root.querySelectorAll("*"))]) {
    if (!holdsText(line)) {
      continue;
    }

    const cut = cutLine(line);

    if (cut !== null) {
      return cut;
    }
  }

  return null;
}

function holdsText(element: Element): boolean {
  for (const child of Array.from(element.childNodes)) {
    if (child.nodeType === Node.TEXT_NODE && (child.textContent ?? "").trim() !== "") {
      return true;
    }
  }

  return false;
}

function clips(element: Element): boolean {
  const style = getComputedStyle(element);

  return CLIPPING.has(style.overflowX) || CLIPPING.has(style.overflowY);
}

// The box a clipping element shows: its own edge plus its padding, with no border or scrollbar.
function contentBox(element: Element): Rect {
  const rect = element.getBoundingClientRect();
  const left = rect.left + element.clientLeft;
  const top = rect.top + element.clientTop;

  return {
    left,
    top,
    right: left + element.clientWidth,
    bottom: top + element.clientHeight,
  };
}

function textBox(line: Element): Rect {
  const range = document.createRange();

  range.selectNodeContents(line);

  const rect = range.getBoundingClientRect();

  return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom };
}

function cutLine(line: Element): Clipped | null {
  const text = textBox(line);

  if (text.right - text.left < 1 && text.bottom - text.top < 1) {
    return null;
  }

  for (let at: Element | null = line; at !== null; at = at.parentElement) {
    if (!clips(at)) {
      continue;
    }

    const box = contentBox(at);

    if (isCut({ text, box }) || (at === line && outgrows(at))) {
      const rect = visiblePart({ text, box });

      if (rect === null) {
        return null;
      }

      const whole = (line as HTMLElement).innerText?.trim() ?? line.textContent?.trim() ?? "";

      return whole === "" ? null : { text: whole, rect };
    }
  }

  return null;
}

// A line that is laid out past its own box even where the range cannot say so, as a block clamped
// to a few lines is.
function outgrows(element: Element): boolean {
  return element.scrollWidth > element.clientWidth + SLACK || element.scrollHeight > element.clientHeight + SLACK;
}
