import type { DrawnPaint, DrawnPath } from "#core";

// How the core's drawing becomes SVG (docs/face.md, 「The face's core」): each path as path data, a
// segment to a cubic command, a closed one closing on its start, and each paint as the attributes that
// lay it down. The display, the editor's canvas and its preview draw with the same functions, so what
// is edited is what is shown.

/** A path as SVG path data: one move and a cubic command a segment, closed where it closes. */
export function pathData(path: DrawnPath): string {
  const curves = path.curves.map(([c1x, c1y, c2x, c2y, x, y]) => ` C ${c1x} ${c1y} ${c2x} ${c2y} ${x} ${y}`).join("");

  return `M ${path.start[0]} ${path.start[1]}${curves}${path.closed ? " Z" : ""}`;
}

/** What one paint is drawn with: its path, its colour as a fill or a round-ended stroke, and its clip. */
export function paintAttributes(paint: DrawnPaint, clipId: (index: number) => string): Record<string, string | number> {
  const attributes: Record<string, string | number> =
    "fill" in paint
      ? { d: pathData(paint.path), fill: paint.fill, stroke: "none" }
      : { d: pathData(paint.path), fill: "none", stroke: paint.stroke, "stroke-width": paint.width, "stroke-linecap": "round", "stroke-linejoin": "round" };

  if (paint.clip !== null) {
    attributes["clip-path"] = `url(#${clipId(paint.clip)})`;
  }

  return attributes;
}
