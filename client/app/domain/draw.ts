import { LIMITS } from "#faces";

import type { FaceColors, Part, Point } from "#faces";

// How a part of the expression document becomes SVG (docs/face.md, 「The faces」 and 「The expression
// document」). Every part is drawn as the same run of commands — one move and as many cubic segments
// as a path can hold — so any part morphs into any other. The display and the editor draw with the
// same function, so what is edited is what is shown.

/** A cubic Bézier segment: its start, its two control points and its end. */
export type Segment = [Point, Point, Point, Point];

/** How many cubic segments every drawn path holds: as many as the most nodes a closed path may have. */
export const DRAWN_SEGMENTS = LIMITS.maxNodes;

const round = (value: number) => Math.round(value * 100) / 100;
const at = ([x, y]: Point) => `${round(x)} ${round(y)}`;
const mix = ([ax, ay]: Point, [bx, by]: Point, t: number): Point => [ax + (bx - ax) * t, ay + (by - ay) * t];

/** The segments a part's nodes make, as authored; a closed path's last one runs back to its first node. */
export function segmentsOf(part: Part): Segment[] {
  const { nodes } = part;
  const count = part.closed ? nodes.length : nodes.length - 1;

  return Array.from({ length: count }, (_, index) => {
    const from = nodes[index]!;
    const to = nodes[(index + 1) % nodes.length]!;

    return [from.anchor, [from.anchor[0] + from.leaving[0], from.anchor[1] + from.leaving[1]], [to.anchor[0] + to.coming[0], to.anchor[1] + to.coming[1]], to.anchor];
  });
}

/** The point a segment passes through at parameter `t`. */
export function pointOn([p0, p1, p2, p3]: Segment, t: number): Point {
  const a = mix(p0, p1, t);
  const b = mix(p1, p2, t);
  const c = mix(p2, p3, t);
  const d = mix(a, b, t);
  const e = mix(b, c, t);

  return mix(d, e, t);
}

/** A segment cut in two at parameter `t` by de Casteljau's construction; together they draw it unchanged. */
export function splitSegment([p0, p1, p2, p3]: Segment, t: number): [Segment, Segment] {
  const a = mix(p0, p1, t);
  const b = mix(p1, p2, t);
  const c = mix(p2, p3, t);
  const d = mix(a, b, t);
  const e = mix(b, c, t);
  const middle = mix(d, e, t);

  return [
    [p0, a, d, middle],
    [middle, e, c, p3],
  ];
}

/** A segment cut into `pieces` of equal parameter. */
function cutEvenly(segment: Segment, pieces: number): Segment[] {
  const cut: Segment[] = [];
  let rest = segment;

  for (let left = pieces; left > 1; left--) {
    const [piece, after] = splitSegment(rest, 1 / left);

    cut.push(piece);
    rest = after;
  }

  cut.push(rest);

  return cut;
}

/**
 * The part's segments cut to exactly `DRAWN_SEGMENTS` of them: each authored segment into an equal
 * share by parameter, the first ones taking one more where the count does not divide.
 */
export function drawnSegments(part: Part): Segment[] {
  const authored = segmentsOf(part);
  const share = Math.floor(DRAWN_SEGMENTS / authored.length);
  const over = DRAWN_SEGMENTS % authored.length;

  return authored.flatMap((segment, index) => cutEvenly(segment, share + (index < over ? 1 : 0)));
}

/** The path data: one move and `DRAWN_SEGMENTS` cubic segments; a closed path returns by its last one. */
export function pathData(part: Part): string {
  const segments = drawnSegments(part);

  return segments.reduce((path, [, c1, c2, end]) => `${path} C ${at(c1)} ${at(c2)} ${at(end)}`, `M ${at(segments[0]![0])}`);
}

/** What one part is drawn as: its path, its paints resolved to colours, and what is drawn inside it. */
export interface DrawnPart {
  d: string;
  fill: string;
  stroke: string;
  strokeWidth: number;
  inside: DrawnPart[];
}

export function drawPart(part: Part, colors: FaceColors): DrawnPart {
  return {
    d: pathData(part),
    fill: part.fill === null ? "none" : colors[part.fill],
    stroke: part.line === null ? "none" : colors[part.line.paint],
    strokeWidth: part.line?.width ?? 0,
    inside: (part.inside ?? []).map((inner) => drawPart(inner, colors)),
  };
}
