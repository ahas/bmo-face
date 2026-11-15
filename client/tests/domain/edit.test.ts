import { describe, expect, test } from "bun:test";

import { partsDrawing } from "#core";
import { DEFAULT_FACES, EMOTIONS, FACE_NAMES, FALLBACK_FACE, LIMITS } from "#faces";

import {
  addInside,
  addMouth,
  addNode,
  changed,
  faceParts,
  handleAt,
  handlesOf,
  isMirrored,
  moveHandle,
  newInside,
  newPart,
  onScreen,
  partAt,
  partsOf,
  reachOf,
  refusalOf,
  refusalWords,
  removeInside,
  removeMouth,
  removeNode,
  saveBody,
  setClosed,
  setMirrored,
  setReach,
  setSize,
  setSmooth,
  withPart,
  withParts,
} from "../../app/domain/edit";

import type { FaceDocument, Part, PathNode, Point } from "#faces";

const K = 0.5523;
const oval = (cx: number, cy: number, rx: number, ry: number): PathNode[] => [
  { anchor: [cx + rx, cy], coming: [0, -ry * K], leaving: [0, ry * K], kind: "smooth" },
  { anchor: [cx, cy + ry], coming: [rx * K, 0], leaving: [-rx * K, 0], kind: "smooth" },
  { anchor: [cx - rx, cy], coming: [0, ry * K], leaving: [0, -ry * K], kind: "smooth" },
  { anchor: [cx, cy - ry], coming: [-rx * K, 0], leaving: [rx * K, 0], kind: "smooth" },
];
const doc = (): FaceDocument => structuredClone(DEFAULT_FACES);
const node = (anchor: Point, coming: Point = [0, 0], leaving: Point = [0, 0], kind: PathNode["kind"] = "corner"): PathNode => ({ anchor, coming, leaving, kind });
const dot: Part = { nodes: oval(30, 30, 4, 3), closed: true, fill: "ink", line: null };
const line: Part = { nodes: [node([10, 10], [-3, -4], [3, 4], "smooth"), node([20, 20], [-3, 0], [3, 0], "smooth"), node([30, 10], [-3, 4], [3, -4], "smooth")], closed: false, fill: null, line: { paint: "ink", width: 0.8 } };
const smooth: Part = { nodes: [node([10, 40]), node([30, 40], [-5, 0], [5, 0], "smooth"), node([50, 40])], closed: false, fill: null, line: { paint: "ink", width: 2 } };
const mouth: Part = { nodes: oval(50, 52, 7, 4), closed: true, fill: "mouth", line: { paint: "ink", width: 0.8 } };
const [calm, joy, sad] = [EMOTIONS[0], EMOTIONS[1], EMOTIONS[2]];

/** Points along a part as the core draws it. */
const samples = (part: Part): Point[] =>
  partsDrawing([part], DEFAULT_FACES.colors).paint[0]!.path.curves.flatMap(([, , , , x, y]) => [[x, y] as Point]);

// The editor's logic (docs/face.md, 「The editor」).
describe("what is being edited", () => {
  test("one side of a face's paired element, another element and a talking mouth are read and replaced apart", () => {
    const brows = { tab: "faces", face: sad, element: "brows", side: "left" } as const;
    const marks = { tab: "faces", face: sad, element: "marks", side: "left" } as const;
    const talk = { tab: "talk", family: 0, mouth: 2 } as const;
    const edited = withParts(withParts(withParts(doc(), brows, [line]), marks, [dot]), talk, [mouth]);

    expect(partsOf(edited, brows)).toEqual([line]);
    expect(partsOf(edited, marks)).toEqual([dot]);
    expect(partsOf(edited, talk)).toEqual([mouth]);
    expect(edited.talk[0]!.mouths[1]).toEqual(DEFAULT_FACES.talk[0]!.mouths[1]!);
    expect(partsOf(edited, { ...brows, element: "eyes" })).toEqual(DEFAULT_FACES.faces[sad]!.eyes.left);
  });

  test("a right side mirrors the left until it is drawn apart, then is edited on its own", () => {
    const right = { tab: "faces", face: calm, element: "eyes", side: "right" } as const;

    expect(isMirrored(doc(), right)).toBe(true);
    expect(partsOf(doc(), right)[0]!.nodes[0]!.anchor[0]).toBeCloseTo(100 - DEFAULT_FACES.faces[calm]!.eyes.left[0]!.nodes[0]!.anchor[0]);

    const apart = setMirrored(doc(), calm, "eyes", false);

    expect(isMirrored(apart, right)).toBe(false);
    expect(partsOf(apart, right)).toEqual(partsOf(doc(), right));
    expect(partsOf(withParts(apart, right, [dot]), right)).toEqual([dot]);
    expect(setMirrored(withParts(apart, right, [dot]), calm, "eyes", true).faces[calm]!.eyes.right).toBeNull();
  });

  test("how far the eyes look is held to its limit", () => {
    expect(setReach(doc(), calm, [7, -1]).faces[calm]!.eyes.reach).toEqual([LIMITS.maxReach, 0]);
  });

  test("a face the document lacks is made from the fallback when it is first edited", () => {
    const { [EMOTIONS[4]]: _gone, ...faces } = doc().faces;
    const edited = withParts({ ...doc(), faces }, { tab: "faces", face: EMOTIONS[4], element: "marks", side: "left" }, [line]);

    expect(edited.faces[EMOTIONS[4]]).toEqual({ ...DEFAULT_FACES.faces[FALLBACK_FACE]!, marks: [line] });
  });

  test("the rest of a face is every other element and side, in drawing order", () => {
    const face = DEFAULT_FACES.faces[joy]!;

    expect(faceParts(face, { element: "mouth", side: null })).toHaveLength(2);
    expect(faceParts(face, { element: "eyes", side: "left" })).toHaveLength(2);
    expect(faceParts(face)).toHaveLength(3);
  });

  test("a part inside another is read and replaced through its outline", () => {
    const withTeeth = addInside(mouth, "teeth");
    const at = { index: 0, inside: 0 };
    const moved = moveHandle(partAt([withTeeth], at)!, { node: 0, kind: "anchor" }, [40, 44]);
    const parts = withPart([withTeeth], at, moved);

    expect(partAt(parts, at)!.nodes[0]!.anchor).toEqual([40, 44]);
    expect(parts[0]!.nodes).toBe(withTeeth.nodes);
  });
});

describe("the handles", () => {
  test("every anchor has one; the chosen node also shows the ends of its handles, but none of no length", () => {
    expect(handlesOf(smooth).map(({ node: at, kind }) => `${at}${kind}`)).toEqual(["0anchor", "1anchor", "2anchor"]);
    expect(handlesOf(smooth, 1).slice(3)).toEqual([
      { node: 1, kind: "coming", at: [25, 40] },
      { node: 1, kind: "leaving", at: [35, 40] },
    ]);
    expect(handlesOf(smooth, 0)).toHaveLength(3);
  });

  test("dragging an anchor carries its handles, stopping where one would leave the screen", () => {
    expect(moveHandle(smooth, { node: 1, kind: "anchor" }, [32.34, 45])).toMatchObject({ nodes: [{}, { anchor: [32.3, 45], coming: [-5, 0], leaving: [5, 0] }, {}] });
    expect(moveHandle(smooth, { node: 1, kind: "anchor" }, [99, 80]).nodes[1]!.anchor).toEqual([95, 75]);
  });

  test("dragging a handle's end on a smooth node turns the other to stay on its line, keeping its length", () => {
    const moved = moveHandle(smooth, { node: 1, kind: "leaving" }, [30, 50]).nodes[1]!;

    expect(moved.leaving).toEqual([0, 10]);
    expect(moved.coming).toEqual([0, -5]);
  });

  test("on a corner, a handle moves alone", () => {
    const cornered = setSmooth(smooth, 1, false);

    expect(moveHandle(cornered, { node: 1, kind: "leaving" }, [30, 50]).nodes[1]).toMatchObject({ leaving: [0, 10], coming: [-5, 0] });
  });

  test("a handle's end never leaves the screen, and the other is shortened to its edge", () => {
    const edge: Part = { ...smooth, nodes: [node([10, 40]), node([2, 40], [-1, 0], [1, 0], "smooth"), node([50, 40])] };
    const moved = moveHandle(edge, { node: 1, kind: "leaving" }, [20, 40]).nodes[1]!;

    expect(moved.leaving).toEqual([18, 0]);
    expect(moved.coming).toEqual([-1, 0]);
    expect(moveHandle(edge, { node: 1, kind: "coming" }, [-8, 40]).nodes[1]).toMatchObject({ coming: [-2, 0], leaving: [1, 0] });
  });

  test("what moves lands on a tenth", () => {
    expect(onScreen([-3, 33.333])).toEqual([0, 33.3]);
    expect(moveHandle(smooth, { node: 1, kind: "coming" }, [21.26, 43.33]).nodes[1]!.coming).toEqual([-8.7, 3.3]);
  });

  test("handles close together are each reached where they stand; apart, a finger's width", () => {
    const small = handlesOf({ ...dot, nodes: oval(30, 30, 1.6, 1.6) });
    const tiny = handlesOf({ ...dot, nodes: oval(30, 30, 0.5, 0.5) });

    expect(reachOf(small, 0)).toBeCloseTo((1.6 * Math.SQRT2) / 2);
    expect(reachOf(tiny, 0)).toBeCloseTo(0.8);
    expect(reachOf(handlesOf(line), 1)).toBe(3.5);
    expect(reachOf(handlesOf(line), 1, 6)).toBeCloseTo(6);
  });

  test("a press takes the nearest handle within reach, and the anchor where a handle stands on it", () => {
    const short: Part = { ...smooth, nodes: [node([10, 40]), node([30, 40], [0, 0], [0.4, 0], "corner"), node([50, 40])] };
    const handles = handlesOf(short, 1);

    expect(handleAt(handles, [30.2, 40])).toEqual({ node: 1, kind: "anchor" });
    expect(handleAt(handles, [30.4, 40])).toEqual({ node: 1, kind: "leaving" });
    expect(handleAt(handles, [49, 41])).toEqual({ node: 2, kind: "anchor" });
    expect(handleAt(handles, [40, 60])).toBeNull();
  });
});

describe("smooth and corner", () => {
  test("made smooth, the coming handle turns opposite the leaving one and keeps its own length", () => {
    const bent: Part = { ...smooth, nodes: [node([10, 40]), node([30, 40], [0, -3], [4, 0]), node([50, 40])] };

    expect(setSmooth(bent, 1, true).nodes[1]).toEqual({ anchor: [30, 40], coming: [-3, 0], leaving: [4, 0], kind: "smooth" });
  });

  test("a coming handle of no length takes the leaving one's", () => {
    const half: Part = { ...smooth, nodes: [node([10, 40]), node([30, 40], [0, 0], [4, 3]), node([50, 40])] };

    expect(setSmooth(half, 1, true).nodes[1]).toMatchObject({ coming: [-4, -3], leaving: [4, 3] });
  });

  test("a sharp corner is pulled out to the handles of the smooth run through its part's anchors", () => {
    const sharp: Part = { ...smooth, nodes: [node([10, 40]), node([30, 46]), node([50, 40])] };

    expect(setSmooth(sharp, 1, true).nodes[1]).toEqual({ anchor: [30, 46], coming: [-6.7, 0], leaving: [6.7, 0], kind: "smooth" });
    // The end of an open run leaves along the parabola its three points make: an arc, not a bend.
    expect(setSmooth(sharp, 0, true).nodes[0]).toMatchObject({ coming: [-6.7, -4], leaving: [6.7, 4] });
  });

  test("made a corner, the handles stay where they are", () => {
    expect(setSmooth(smooth, 1, false).nodes[1]).toEqual({ ...smooth.nodes[1]!, kind: "corner" });
  });
});

describe("nodes added and removed", () => {
  const onDrawing = (before: Part, after: Part) => {
    const was = partsDrawing([before], DEFAULT_FACES.colors).paint[0]!.path;
    const fine = Array.from({ length: was.curves.length * 50 + 1 }, (_, step) => step / 50).map((at) => {
      const index = Math.min(was.curves.length - 1, Math.floor(at));
      const [c1x, c1y, c2x, c2y, x, y] = was.curves[index]!;
      const [sx, sy] = index === 0 ? was.start : was.curves[index - 1]!.slice(4);
      const t = at - index;
      const u = 1 - t;

      return [u * u * u * sx! + 3 * u * u * t * c1x + 3 * u * t * t * c2x + t * t * t * x, u * u * u * sy! + 3 * u * u * t * c1y + 3 * u * t * t * c2y + t * t * t * y];
    });

    for (const point of samples(after)) {
      expect(Math.min(...fine.map(([x, y]) => Math.hypot(x! - point[0], y! - point[1])))).toBeLessThan(0.05);
    }
  };

  test("a node is added after the chosen one at the middle of the segment it leaves by, the drawing unchanged", () => {
    const { part, chosen } = addNode(line, 0);

    expect(part.nodes).toHaveLength(4);
    expect(chosen).toBe(1);
    expect(part.nodes[1]!.kind).toBe("smooth");
    onDrawing(line, part);
  });

  test("at the end of an open path, the segment coming into it is cut", () => {
    const { part, chosen } = addNode(line, 2);

    expect(chosen).toBe(2);
    expect(part.nodes[3]!.anchor).toEqual([30, 10]);
    onDrawing(line, part);
  });

  test("on a closed path, the last node's segment runs back to the first", () => {
    const { part, chosen } = addNode(dot, 3);

    expect(chosen).toBe(4);
    expect(part.nodes).toHaveLength(5);
    onDrawing(dot, part);
  });

  test("none is added past the limit, and nodes are removed down to two", () => {
    const full: Part = { ...line, nodes: Array.from({ length: LIMITS.maxNodes }, (_, at) => node([at, 10])) };

    expect(addNode(full, 3).part).toBe(full);
    expect(removeNode(line, 1).nodes.map((own) => own.anchor)).toEqual([[10, 10], [30, 10]]);
    expect(removeNode(removeNode(line, 1), 0).nodes).toHaveLength(2);
  });
});

describe("open and closed", () => {
  test("opening drops the fill, and a part with no line takes its fill's paint as one", () => {
    expect(setClosed(dot, false)).toMatchObject({ closed: false, fill: null, line: { paint: "ink", width: 1 } });
    expect(setClosed(mouth, false)).toMatchObject({ closed: false, fill: null, line: { paint: "ink", width: 0.8 } });
  });

  test("a part with anything inside it is not opened", () => {
    expect(setClosed(addInside(mouth, "teeth"), false)).toBeNull();
  });

  test("closing keeps the part's paints", () => {
    expect(setClosed(line, true)).toEqual({ ...line, closed: true });
  });
});

describe("new parts", () => {
  test("an oval and a line bring their own paints, whatever element they go in", () => {
    expect(newPart("oval")).toMatchObject({ closed: true, fill: "ink", line: null });
    expect(newPart("line")).toMatchObject({ closed: false, fill: null, line: { paint: "ink", width: 0.8 } });
  });

  test("inside an opening, each part comes in its own paint, the teeth along its top and the tongue from its bottom", () => {
    const top = (part: Part) => Math.min(...part.nodes.map((own) => own.anchor[1]));

    expect(newInside("teeth", mouth)).toMatchObject({ closed: true, fill: "light", line: null });
    expect(top(newInside("teeth", mouth))).toBeLessThan(48);
    expect(newInside("lowerTeeth", mouth).fill).toBe("light");
    expect(top(newInside("lowerTeeth", mouth))).toBeGreaterThan(50);
    expect(newInside("tongue", mouth).fill).toBe("tongue");
    expect(newInside("tongueLight", mouth).fill).toBe("tongueLight");
    expect(newInside("shine", dot).fill).toBe("light");
  });

  test("every new part is one the check takes, in any element or talking mouth", () => {
    const opening = (["teeth", "lowerTeeth", "tongueLight", "tongue"] as const).reduce(addInside, mouth);

    for (const target of [{ tab: "talk", family: 1, mouth: 0 }, { tab: "faces", face: joy, element: "blush", side: "left" }] as const) {
      expect(refusalOf(withParts(doc(), target, [newPart("oval"), newPart("line"), opening]))).toBeNull();
    }
  });

  test("parts go inside a closed part only, up to its limit, and come out again", () => {
    let full = mouth;

    for (let i = 0; i < 10; i++) full = addInside(full, "tongue");
    expect(full.inside).toHaveLength(LIMITS.inside);
    expect(addInside(line, "teeth")).toBe(line);
    expect(removeInside(addInside(mouth, "teeth"), 0)).toEqual(mouth);
    expect(removeInside(full, 0).inside).toHaveLength(LIMITS.inside - 1);
  });
});

describe("the talking mouth set", () => {
  test("a mouth is added to a family as a copy after the one chosen, and removed down to one", () => {
    const added = addMouth(doc(), 1, 0);

    expect(added.talk[1]!.mouths).toHaveLength(4);
    expect(added.talk[1]!.mouths[1]).toEqual(DEFAULT_FACES.talk[1]!.mouths[0]!);

    let fewer = doc();

    for (let i = 0; i < 20; i++) fewer = removeMouth(fewer, 1, 0);
    expect(fewer.talk[1]!.mouths).toHaveLength(1);
    expect(setSize(doc(), 1, 0, "large").talk[1]!.mouths[0]!.size).toBe("large");
  });
});

describe("saving", () => {
  test("sends the document without its version, with the version it was edited from", () => {
    const body = saveBody({ ...doc(), savedAt: 77 }, 77);

    expect(body.basedOn).toBe(77);
    expect(body.document).not.toHaveProperty("savedAt");
  });

  test("a document is changed only where what it draws differs, not its version", () => {
    expect(changed({ ...doc(), savedAt: 5 }, doc())).toBe(false);
    expect(changed(withParts(doc(), { tab: "talk", family: 0, mouth: 0 }, [dot]), doc())).toBe(true);
  });

  test("a document is not changed by the order its keys were written in", () => {
    const saved = doc();
    const name = FACE_NAMES[0]!;
    const face = saved.faces[name]!;
    const reordered: FaceDocument = {
      ...saved,
      faces: { ...saved.faces, [name]: { mouth: face.mouth, marks: face.marks, blush: face.blush, eyes: face.eyes, brows: face.brows } },
    };

    expect(changed(reordered, saved)).toBe(false);
  });

  test("a document the server would refuse is refused here first, and worded from the language pack", () => {
    expect(refusalOf(doc())).toBeNull();

    const refusal = refusalOf(withParts(doc(), { tab: "faces", face: calm, element: "mouth", side: "left" }, [{ ...line, line: { paint: "ink", width: 20 } }]))!;

    expect(refusal).toEqual({ code: "line-width", at: [{ face: calm }, { element: "mouth" }, { part: 1 }], least: 0.2, most: 10 });

    const said: string[] = [];
    const t = (key: string, values?: Record<string, unknown>) => {
      said.push(key);

      return `${key}${values?.n ?? ""}`;
    };

    expect(refusalWords(refusal, t)).toBe(`face.name.${calm} · face.edit.element.mouth · face.edit.place.part1: face.edit.refusal.line-width`);
  });
});
