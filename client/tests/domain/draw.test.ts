import { describe, expect, test } from "bun:test";

import { partsDrawing } from "#core";
import { DEFAULT_FACES, EMOTIONS } from "#faces";

import { paintAttributes, pathData } from "../../app/domain/draw";

import type { DrawnPath } from "#core";
import type { Part } from "#faces";

// How the core's drawing becomes SVG (docs/face.md, 「The face's core」).
describe("a drawn path", () => {
  const triangle: DrawnPath = { start: [40, 50], curves: [[46, 50, 54, 50, 60, 50], [57, 53, 53, 57, 50, 60]], closed: true };

  test("is one move and a cubic command a segment, closing where it closes", () => {
    expect(pathData(triangle)).toBe("M 40 50 C 46 50 54 50 60 50 C 57 53 53 57 50 60 Z");
    expect(pathData({ ...triangle, closed: false })).toBe("M 40 50 C 46 50 54 50 60 50 C 57 53 53 57 50 60");
  });

  test("holds as many segments as its shape, never a fixed count", () => {
    for (const face of Object.values(DEFAULT_FACES.faces)) {
      for (const paint of partsDrawing(face.mouth, DEFAULT_FACES.colors).paint) {
        expect(pathData(paint.path).match(/C/g)?.length).toBe(paint.path.curves.length);
        expect(paint.path.curves.length).toBeLessThan(10);
      }
    }
  });
});

describe("a paint", () => {
  test("fills with its colour, or strokes with round ends and joins, cut off at its clip", () => {
    const mouth = DEFAULT_FACES.faces[EMOTIONS[1]]!.mouth as Part[];
    const drawing = partsDrawing(mouth, DEFAULT_FACES.colors);
    const [cavity, ...rest] = drawing.paint;
    const outline = rest.at(-1)!;
    const inside = rest.slice(0, -1);
    const clip = (index: number) => `clip-${index}`;

    expect(paintAttributes(cavity!, clip)).toMatchObject({ fill: DEFAULT_FACES.colors.mouth, stroke: "none" });
    expect(paintAttributes(outline, clip)).toMatchObject({ fill: "none", stroke: DEFAULT_FACES.colors.ink, "stroke-linecap": "round", "stroke-linejoin": "round" });
    expect(inside.length).toBeGreaterThan(0);
    expect(inside.every((paint) => paintAttributes(paint, clip)["clip-path"] === "url(#clip-0)")).toBe(true);
  });
});
