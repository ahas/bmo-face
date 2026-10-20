import { describe, expect, test } from "bun:test";

import { checkFaceDocument, DEFAULT_FACES, EMOTIONS, faceFor, FACE_NAMES, ovalNodes, OVAL_HANDLE, PAINTS, PART_GROUPS, splineNodes, STATES } from "../src/faces";

import type { Part } from "../src/faces";

// The expression document (docs/face.md, 「The expression document」).
describe("the default document", () => {
  test("holds a face for every emotion and every state, and passes its own check", () => {
    expect(Object.keys(DEFAULT_FACES.faces).sort()).toEqual([...FACE_NAMES].sort());
    expect(checkFaceDocument(DEFAULT_FACES)).toEqual({ ok: { colors: DEFAULT_FACES.colors, faces: DEFAULT_FACES.faces, mouths: DEFAULT_FACES.mouths } });
  });

  test("names the emotions BMO speaks with and the presences it is in", () => {
    expect(EMOTIONS).toEqual(["기본", "기쁨", "슬픔", "놀람", "화남", "졸림", "폭소", "걱정", "아픔", "안간힘"]);
    expect(STATES).toEqual(["writing", "thinking", "idle"]);
  });
});

describe("what the default draws", () => {
  const faces = Object.values(DEFAULT_FACES.faces);
  const parts = [...faces.flatMap((face) => PART_GROUPS.flatMap((group) => face[group])), ...DEFAULT_FACES.mouths.flat()];
  const everyPart = parts.flatMap((part) => [part, ...(part.inside ?? [])]);

  // The middle of a part's anchors, and how far they spread across.
  const centre = (part: Part): [number, number] => [
    part.nodes.reduce((sum, node) => sum + node.anchor[0], 0) / part.nodes.length,
    part.nodes.reduce((sum, node) => sum + node.anchor[1], 0) / part.nodes.length,
  ];
  const span = (parts: Part[]) => {
    const xs = parts.flatMap((part) => part.nodes.map((node) => node.anchor[0]));

    return [Math.min(...xs), Math.max(...xs)] as const;
  };

  test("paints in ink, teeth, the inside of the mouth and the tongue, and draws no cheeks and no tears", () => {
    const used = new Set(everyPart.flatMap((part) => [part.fill, part.line?.paint]).filter((paint) => paint !== null && paint !== undefined));

    expect([...used].sort()).toEqual(["ink", "light", "mouth", "tongue"]);
    expect(PAINTS).toContain("cheek");
    expect(faces.every((face) => face.cheeks.length === 0 && face.marks.length === 0)).toBe(true);
  });

  test("stands every dot eye a quarter of the way in from its side and a third of the way down", () => {
    const dotEyes = faces.flatMap((face) => face.eyes.filter((part) => part.closed && part.fill === "ink"));

    expect(dotEyes.length).toBeGreaterThan(10);

    for (const eye of dotEyes) {
      const [x, y] = centre(eye);

      expect(Math.min(Math.abs(x - 23.5), Math.abs(x - 76.5))).toBeLessThanOrEqual(2);
      expect(Math.abs(y - 25)).toBeLessThanOrEqual(2);
    }
  });

  test("centres every mouth between the eyes, below them, and no wider than the laughing mouth", () => {
    for (const mouth of [...faces.map((face) => face.mouth), ...DEFAULT_FACES.mouths]) {
      const [left, right] = span(mouth);

      expect(Math.abs((left + right) / 2 - 50)).toBeLessThanOrEqual(1.5);
      expect(right - left).toBeLessThanOrEqual(40);
      expect(Math.min(...mouth.flatMap((part) => part.nodes.map((node) => node.anchor[1])))).toBeGreaterThan(33);
    }
  });

  test("closes the first mouth stage on the calm arc, and opens the later ones as the smile growing to 기쁨's", () => {
    const [closed, ...open] = DEFAULT_FACES.mouths;

    expect(closed).toEqual(DEFAULT_FACES.faces.기본!.mouth);
    expect(open).toHaveLength(4);
    expect(open.at(-1)).toEqual(DEFAULT_FACES.faces.기쁨!.mouth);

    const widths = open.map((stage) => span(stage)[1] - span(stage)[0]);

    expect(widths).toEqual([...widths].sort((a, b) => a - b));

    for (const stage of open) {
      expect(stage.map((part) => [part.fill, part.line?.paint, part.inside?.map((inner) => inner.fill)])).toEqual([["mouth", "ink", ["light", "tongue"]]]);
    }
  });

  test("opens every open mouth on the teeth band and the tongue", () => {
    for (const name of ["기쁨", "폭소", "슬픔", "놀람", "화남"] as const) {
      expect(DEFAULT_FACES.faces[name]!.mouth.map((part) => part.inside?.map((inner) => inner.fill))).toEqual([["light", "tongue"]]);
    }
  });

  test("draws the worried strip in teeth, the pained crescent under a teeth band, and the clench in both rows", () => {
    const mouth = (name: "걱정" | "아픔" | "안간힘") =>
      DEFAULT_FACES.faces[name]!.mouth.map((part) => [part.fill, part.line?.paint, part.inside?.map((inner) => inner.fill ?? inner.line?.paint)]);

    expect(mouth("걱정")).toEqual([["light", "ink", undefined]]);
    expect(mouth("아픔")).toEqual([["mouth", "ink", ["light"]]]);
    expect(mouth("안간힘")).toEqual([["mouth", "ink", ["light", "light"]]]);
  });

  test("gives the pained face x x eyes of straight crossing strokes, and the worried one raised brows", () => {
    const eyes = DEFAULT_FACES.faces.아픔!.eyes;

    expect(eyes).toHaveLength(4);
    expect(eyes.every((part) => !part.closed && part.nodes.length === 2)).toBe(true);
    expect(DEFAULT_FACES.faces.걱정!.brows.map((part) => part.nodes[0]!.anchor[1] > part.nodes[1]!.anchor[1])).toEqual([true, false]);
  });

  test("keeps the calm faces to dots and lines", () => {
    for (const name of ["기본", "졸림", "writing", "thinking", "idle"] as const) {
      expect(DEFAULT_FACES.faces[name]!.mouth.every((part) => !part.closed && part.fill === null)).toBe(true);
    }
  });
});

describe("a face that is missing", () => {
  test("falls back to 기본", () => {
    const { 슬픔: _gone, ...rest } = DEFAULT_FACES.faces;
    const document = { ...DEFAULT_FACES, faces: rest };

    expect(faceFor(document, "슬픔")).toBe(DEFAULT_FACES.faces.기본!);
    expect(faceFor(document, "기쁨")).toBe(DEFAULT_FACES.faces.기쁨!);
  });
});

describe("nodes built from shapes", () => {
  test("the spline through points leaves each node by a sixth of the step between its neighbours", () => {
    expect(splineNodes([[10, 10], [20, 16], [30, 10]], false)).toEqual([
      { anchor: [10, 10], coming: [-10 / 6, -1], leaving: [10 / 6, 1], kind: "smooth" },
      { anchor: [20, 16], coming: [-20 / 6, 0], leaving: [20 / 6, 0], kind: "smooth" },
      { anchor: [30, 10], coming: [-10 / 6, 1], leaving: [10 / 6, -1], kind: "smooth" },
    ]);
  });

  test("a closed spline's neighbours run round", () => {
    const [first] = splineNodes([[10, 10], [20, 10], [15, 20]], true);

    expect(first).toEqual({ anchor: [10, 10], coming: [(15 - 20) / 6, (20 - 10) / 6], leaving: [(20 - 15) / 6, (10 - 20) / 6], kind: "smooth" });
  });

  test("an oval is four smooth nodes at the ends of its radii", () => {
    const nodes = ovalNodes(50, 30, 4, 2);

    expect(nodes.map((node) => node.anchor)).toEqual([[54, 30], [50, 32], [46, 30], [50, 28]]);
    expect(nodes[0]!.leaving).toEqual([0, 2 * OVAL_HANDLE]);
    expect(nodes[1]!.leaving).toEqual([-4 * OVAL_HANDLE, 0]);
  });
});

const corner = (x: number, y: number) => ({ anchor: [x, y], coming: [0, 0], leaving: [0, 0], kind: "corner" });
const stroke = { paint: "ink", width: 2 };
const triangle = (extra: Record<string, unknown> = {}) => ({ nodes: [corner(40, 50), corner(60, 50), corner(50, 60)], closed: true, fill: "mouth", line: stroke, ...extra });
const face = (mouth: unknown[]) => ({ eyes: [], brows: [], cheeks: [], mouth, marks: [] });
const withMouth = (...mouth: unknown[]) => ({ ...DEFAULT_FACES, faces: { ...DEFAULT_FACES.faces, 기쁨: face(mouth) } });

describe("the check refuses a document whole", () => {
  test("with no 기본 face, since every missing face falls back to it", () => {
    const { 기본: _gone, ...rest } = DEFAULT_FACES.faces;

    expect(checkFaceDocument({ ...DEFAULT_FACES, faces: rest })).toEqual({ error: "기본 표정은 지울 수 없어요." });
  });

  test("with a face of a name nobody sends", () => {
    expect(checkFaceDocument({ ...DEFAULT_FACES, faces: { ...DEFAULT_FACES.faces, 신남: DEFAULT_FACES.faces.기본 } })).toEqual({ error: "모르는 표정 이름이에요: 신남" });
  });

  test("with a part of neither a fill nor a line", () => {
    expect(checkFaceDocument(withMouth(triangle({ fill: null, line: null })))).toEqual({ error: "기쁨 mouth 1: 채우기나 선 가운데 하나는 있어야 해요." });
  });

  test("with an open part that is filled", () => {
    expect(checkFaceDocument(withMouth(triangle({ closed: false })))).toEqual({ error: "기쁨 mouth 1: 열린 선은 채울 수 없어요." });
  });

  test("with parts inside an open part", () => {
    expect(checkFaceDocument(withMouth(triangle({ closed: false, fill: null, inside: [triangle()] })))).toEqual({ error: "기쁨 mouth 1: 열린 선 안에는 모양을 넣을 수 없어요." });
  });

  test("with parts inside a part inside", () => {
    expect(checkFaceDocument(withMouth(triangle({ inside: [triangle({ inside: [] })] })))).toEqual({ error: "기쁨 mouth 1 안 1: 안에 넣은 모양 안에는 다시 넣을 수 없어요." });
  });

  test("with more parts inside one than it may hold", () => {
    expect(checkFaceDocument(withMouth(triangle({ inside: Array.from({ length: 9 }, () => triangle()) })))).toEqual({ error: "기쁨 mouth 1: 안에 넣는 모양은 8개까지예요." });
  });

  test("with an anchor outside the screen", () => {
    expect(checkFaceDocument(withMouth(triangle({ nodes: [corner(40, 50), corner(101, 50)] })))).toEqual({ error: "기쁨 mouth 1 노드 2: 노드나 손잡이가 화면 밖으로 나가요." });
  });

  test("with a handle whose end falls outside the screen, though its anchor is on it", () => {
    const pulled = { anchor: [2, 50], coming: [0, 0], leaving: [-3, 0], kind: "smooth" };

    expect(checkFaceDocument(withMouth(triangle({ nodes: [pulled, corner(60, 50)] })))).toEqual({ error: "기쁨 mouth 1 노드 1: 노드나 손잡이가 화면 밖으로 나가요." });
  });

  test("with one node, or a node that is not an anchor and two handles", () => {
    expect(checkFaceDocument(withMouth(triangle({ nodes: [corner(40, 50)] })))).toEqual({ error: "기쁨 mouth 1: 노드는 2개에서 32개 사이여야 해요." });
    expect(checkFaceDocument(withMouth(triangle({ nodes: [corner(40, 50), { anchor: [1, "x"], coming: [0, 0], leaving: [0, 0], kind: "corner" }] })))).toEqual({ error: "기쁨 mouth 1 노드 2: 노드의 값이 맞지 않아요." });
  });

  test("with a paint the document does not hold, or a line too thin", () => {
    expect(checkFaceDocument(withMouth(triangle({ fill: "red" })))).toEqual({ error: "기쁨 mouth 1: 채우기 칠이 맞지 않아요." });
    expect(checkFaceDocument(withMouth(triangle({ line: { paint: "ink", width: 0.1 } })))).toEqual({ error: "기쁨 mouth 1: 선 굵기는 0.2에서 10 사이여야 해요." });
  });

  test("with a part in the old ellipse shape", () => {
    expect(checkFaceDocument(withMouth({ shape: "ellipse", cx: 50, cy: 50, rx: 3, ry: 3, filled: true, width: 1 }))).toHaveProperty("error");
  });

  test("with too few or too many mouth stages", () => {
    expect(checkFaceDocument({ ...DEFAULT_FACES, mouths: [[]] })).toEqual({ error: "입 모양 단계는 2개에서 8개 사이여야 해요." });
    expect(checkFaceDocument({ ...DEFAULT_FACES, mouths: Array.from({ length: 9 }, () => []) })).toHaveProperty("error");
  });

  test("with a colour that is not #rrggbb, or a paint's colour missing", () => {
    expect(checkFaceDocument({ ...DEFAULT_FACES, colors: { ...DEFAULT_FACES.colors, body: "teal" } })).toEqual({ error: "색 body는 #rrggbb 꼴이어야 해요." });

    const { tongue: _gone, ...colors } = DEFAULT_FACES.colors;

    expect(checkFaceDocument({ ...DEFAULT_FACES, colors })).toEqual({ error: "색 tongue는 #rrggbb 꼴이어야 해요." });
  });
});

describe("the check passes", () => {
  test("an open mouth with a teeth band and an open divider inside, in a mouth stage too", () => {
    const band = triangle({ fill: "light", line: null });
    const divider = { nodes: [corner(50, 50), corner(50, 55)], closed: false, fill: null, line: { paint: "ink", width: 0.6 } };
    const mouth = triangle({ inside: [band, divider] });
    const checked = checkFaceDocument({ ...withMouth(mouth), mouths: [[mouth], [mouth]] });

    expect("ok" in checked && checked.ok.faces.기쁨!.mouth).toEqual([mouth as Part]);
    expect("ok" in checked && checked.ok.mouths[1]).toEqual([mouth as Part]);
  });
});

describe("the check keeps only what it knows", () => {
  test("fields it does not know are dropped, an empty inside with them, and a missing group is empty", () => {
    const checked = checkFaceDocument({ ...DEFAULT_FACES, faces: { ...DEFAULT_FACES.faces, 기쁨: { eyes: [triangle({ colour: "red", inside: [] })], extra: 1 } } });

    expect("ok" in checked && checked.ok.faces.기쁨).toEqual({ eyes: [triangle() as Part], brows: [], cheeks: [], mouth: [], marks: [] });
  });
});
