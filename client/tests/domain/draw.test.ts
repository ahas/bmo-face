import { describe, expect, test } from "bun:test";

import { DEFAULT_FACES, ovalNodes, splineNodes } from "#faces";

import { DRAWN_SEGMENTS, drawnSegments, drawPart, pathData, pointOn, segmentsOf } from "../../app/domain/draw";

import type { Part, PathNode, Point } from "#faces";

// How a part becomes SVG (docs/face.md, 「The faces」 and 「The expression document」).

const corner = (x: number, y: number): PathNode => ({ anchor: [x, y], coming: [0, 0], leaving: [0, 0], kind: "corner" });
const arc: Part = { nodes: splineNodes([[40, 47], [50, 53], [60, 47]], false), closed: false, fill: null, line: { paint: "ink", width: 2 } };
const oval: Part = { nodes: ovalNodes(33, 30, 3, 4), closed: true, fill: "ink", line: null };
const triangle: Part = { nodes: [corner(40, 50), corner(60, 50), corner(50, 60)], closed: true, fill: "mouth", line: { paint: "ink", width: 1.6 } };
const commands = (d: string) => d.replace(/[^A-Za-z]/g, "");

const near = ([ax, ay]: Point, [bx, by]: Point) => {
  expect(ax).toBeCloseTo(bx, 9);
  expect(ay).toBeCloseTo(by, 9);
};

describe("a drawn path", () => {
  test("is one move and the same number of cubic segments, whatever the part", () => {
    const parts = [arc, oval, triangle, { ...triangle, nodes: triangle.nodes.slice(0, 2), closed: false, fill: null }];
    const every = [...parts, ...Object.values(DEFAULT_FACES.faces).flatMap((face) => Object.values(face).flat()), ...DEFAULT_FACES.mouths.flat()];
    const expected = `M${"C".repeat(DRAWN_SEGMENTS)}`;

    for (const part of every) {
      expect(commands(pathData(part))).toBe(expected);
    }
  });

  test("draws the curve its nodes make: each authored segment cut evenly by parameter", () => {
    const [first, second] = segmentsOf(arc);
    const pieces = drawnSegments(arc);
    const share = DRAWN_SEGMENTS / 2;

    for (let piece = 0; piece < share; piece++) {
      near(pointOn(pieces[piece]!, 0.5), pointOn(first!, (piece + 0.5) / share));
      near(pointOn(pieces[share + piece]!, 0.3), pointOn(second!, (piece + 0.3) / share));
    }
  });

  test("gives the first segments one piece more where the count does not divide, and joins them end to end", () => {
    const pieces = drawnSegments(triangle);

    expect(pieces).toHaveLength(DRAWN_SEGMENTS);
    near(pieces[10]![3], [60, 50]);
    near(pieces[21]![3], [50, 60]);
    pieces.slice(1).forEach((piece, index) => near(piece[0], pieces[index]![3]));
  });

  test("closed, returns to its start by a segment and never a close", () => {
    const d = pathData(oval);

    expect(d.startsWith("M 36 30 ")).toBe(true);
    expect(d.endsWith(" 36 30")).toBe(true);
    expect(d).not.toContain("Z");
  });

  test("of two corner nodes is a straight line", () => {
    const line: Part = { nodes: [corner(10, 10), corner(30, 20)], closed: false, fill: null, line: { paint: "ink", width: 2 } };
    const onLine = ([x, y]: Point) => (x - 10) * (20 - 10) - (y - 10) * (30 - 10);

    for (const piece of drawnSegments(line)) {
      piece.forEach((point) => expect(onLine(point)).toBeCloseTo(0, 9));
    }
  });
});

describe("a drawn part", () => {
  test("takes its colours from its own paints, and draws what stands inside it", () => {
    const band: Part = { ...triangle, fill: "light", line: null };
    const drawn = drawPart({ ...triangle, inside: [band] }, DEFAULT_FACES.colors);

    expect(drawn).toMatchObject({ fill: DEFAULT_FACES.colors.mouth, stroke: "#000a00", strokeWidth: 1.6 });
    expect(drawn.inside).toEqual([{ d: pathData(band), fill: "#fcfcfc", stroke: "none", strokeWidth: 0, inside: [] }]);
  });

  test("with no fill is not painted, and with no line is not stroked", () => {
    expect(drawPart(arc, DEFAULT_FACES.colors)).toMatchObject({ fill: "none", stroke: "#000a00", strokeWidth: 2, inside: [] });
    expect(drawPart(oval, DEFAULT_FACES.colors)).toMatchObject({ fill: "#000a00", stroke: "none" });
  });
});
