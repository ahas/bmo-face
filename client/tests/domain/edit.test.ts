import { describe, expect, test } from "bun:test";

import { checkFaceDocument, DEFAULT_FACES, LIMITS, ovalNodes, splineNodes } from "#faces";

import { drawnSegments, pointOn } from "../../app/domain/draw";
import {
  addInside,
  addNode,
  addStage,
  changed,
  handleAt,
  handlesOf,
  moveHandle,
  newInside,
  newPart,
  onScreen,
  partAt,
  partsOf,
  reachOf,
  refusalOf,
  removeInside,
  removeNode,
  removeStage,
  saveBody,
  setClosed,
  setSmooth,
  withPart,
  withParts,
} from "../../app/domain/edit";

import type { FaceDocument, Part, PathNode, Point } from "#faces";

const doc = (): FaceDocument => structuredClone(DEFAULT_FACES);
const node = (anchor: Point, coming: Point = [0, 0], leaving: Point = [0, 0], kind: PathNode["kind"] = "corner"): PathNode => ({ anchor, coming, leaving, kind });
const dot: Part = { nodes: ovalNodes(30, 30, 4, 3), closed: true, fill: "ink", line: null };
const line: Part = { nodes: splineNodes([[10, 10], [20, 20], [30, 10]], false), closed: false, fill: null, line: { paint: "ink", width: 2 } };
const smooth: Part = { nodes: [node([10, 40]), node([30, 40], [-5, 0], [5, 0], "smooth"), node([50, 40])], closed: false, fill: null, line: { paint: "ink", width: 2 } };
const mouth: Part = { nodes: ovalNodes(50, 52, 7, 4), closed: true, fill: "mouth", line: { paint: "ink", width: 1.6 } };

// The editor's logic (docs/face.md, 「The editor」).
describe("what is being edited", () => {
  test("a face's group and a mouth stage are read and replaced apart", () => {
    const face = { tab: "faces", face: "슬픔", group: "brows" } as const;
    const stage = { tab: "mouths", stage: 2 } as const;
    const edited = withParts(withParts(doc(), face, [line]), stage, [dot]);

    expect(partsOf(edited, face)).toEqual([line]);
    expect(partsOf(edited, stage)).toEqual([dot]);
    expect(partsOf(edited, { tab: "faces", face: "슬픔", group: "eyes" })).toEqual(DEFAULT_FACES.faces.슬픔!.eyes);
    expect(edited.mouths[1]).toEqual(DEFAULT_FACES.mouths[1]!);
  });

  test("a face the document lacks is made from 기본 when it is first edited", () => {
    const { 화남: _gone, ...faces } = doc().faces;
    const edited = withParts({ ...doc(), faces }, { tab: "faces", face: "화남", group: "brows" }, [line]);

    expect(edited.faces.화남).toEqual({ ...DEFAULT_FACES.faces.기본!, brows: [line] });
  });

  test("a mouth stage holds several parts, each edited on its own", () => {
    const stage = { tab: "mouths", stage: 1 } as const;
    const edited = withParts(doc(), stage, [mouth, dot, line]);
    const parts = withPart(partsOf(edited, stage), { index: 1, inside: null }, moveHandle(dot, { node: 0, kind: "anchor" }, [40, 30]));

    expect(parts).toHaveLength(3);
    expect(parts[0]).toBe(mouth);
    expect(parts[1]!.nodes[0]!.anchor).toEqual([40, 30]);
    expect(parts[2]).toBe(line);
    expect(refusalOf(withParts(edited, stage, parts))).toBeNull();
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
    const small = handlesOf({ ...dot, nodes: ovalNodes(30, 30, 1.6, 1.6) });
    const tiny = handlesOf({ ...dot, nodes: ovalNodes(30, 30, 0.5, 0.5) });

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

  test("a sharp corner is pulled out to the spline's handles through its neighbours", () => {
    const sharp: Part = { ...smooth, nodes: [node([10, 40]), node([30, 46]), node([50, 40])] };

    expect(setSmooth(sharp, 1, true).nodes[1]).toEqual({ anchor: [30, 46], coming: [-6.7, 0], leaving: [6.7, 0], kind: "smooth" });
    expect(setSmooth(sharp, 0, true).nodes[0]).toMatchObject({ coming: [-3.3, -1], leaving: [3.3, 1] });
  });

  test("made a corner, the handles stay where they are", () => {
    expect(setSmooth(smooth, 1, false).nodes[1]).toEqual({ ...smooth.nodes[1]!, kind: "corner" });
  });
});

describe("nodes added and removed", () => {
  const samples = (part: Part) => drawnSegments(part).flatMap((segment) => [0, 0.5].map((t) => pointOn(segment, t)));
  const onDrawing = (before: Part, after: Part) => {
    const was = samples(before);

    for (const point of samples(after)) {
      const nearest = Math.min(...was.map(([x, y]) => Math.hypot(x - point[0], y - point[1])));

      expect(nearest).toBeLessThan(0.6);
    }
  };

  test("a node is added after the chosen one at the middle of the segment it leaves by, the drawing unchanged", () => {
    const { part, chosen } = addNode(line, 0);

    expect(part.nodes).toHaveLength(4);
    expect(chosen).toBe(1);
    expect(part.nodes[1]!.kind).toBe("smooth");
    expect(pointOn([part.nodes[0]!.anchor, [0, 0], [0, 0], part.nodes[1]!.anchor], 0)).toEqual([10, 10]);
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

  test("a straight corner segment stays straight", () => {
    const straight: Part = { ...smooth, nodes: [node([10, 40]), node([50, 40])] };
    const { part } = addNode(straight, 0);

    expect(part.nodes.map((own) => own.anchor)).toEqual([[10, 40], [30, 40], [50, 40]]);
    expect(part.nodes.every((own) => own.coming[1] === 0 && own.leaving[1] === 0)).toBe(true);
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
    expect(setClosed(mouth, false)).toMatchObject({ closed: false, fill: null, line: { paint: "ink", width: 1.6 } });
  });

  test("a part with anything inside it is not opened", () => {
    expect(setClosed(addInside(mouth, "teeth"), false)).toBeNull();
  });

  test("closing keeps the part's paints", () => {
    expect(setClosed(line, true)).toEqual({ ...line, closed: true });
  });
});

describe("new parts", () => {
  test("an oval and a line bring their own paints, whatever group they go in", () => {
    expect(newPart("oval")).toMatchObject({ closed: true, fill: "ink", line: null });
    expect(newPart("line")).toMatchObject({ closed: false, fill: null, line: { paint: "ink", width: 2.2 } });
  });

  test("inside an opening, the teeth band is light along its top and the tongue a dome from its bottom", () => {
    const teeth = newInside("teeth", mouth);
    const tongue = newInside("tongue", mouth);
    const topOf = (part: Part) => Math.min(...part.nodes.map((own) => own.anchor[1]));

    expect(teeth).toMatchObject({ closed: true, fill: "light", line: null });
    expect(topOf(teeth)).toBeLessThan(48);
    expect(tongue).toMatchObject({ closed: true, fill: "tongue", line: null });
    expect(tongue.nodes[0]!.anchor[1]).toBeGreaterThan(56);
    expect(tongue.nodes[1]!.anchor[0]).toBeLessThan(50);
  });

  test("every new part is one the check takes, in any group or stage", () => {
    const opening = addInside(addInside(mouth, "teeth"), "tongue");

    for (const target of [{ tab: "mouths", stage: 0 }, { tab: "faces", face: "기쁨", group: "cheeks" }] as const) {
      expect(checkFaceDocument(withParts(doc(), target, [newPart("oval"), newPart("line"), opening]))).toHaveProperty("ok");
    }
  });

  test("parts go inside a closed part only, up to its limit, and come out again", () => {
    let full = mouth;

    for (let i = 0; i < 10; i++) full = addInside(full, "tongue");
    expect(full.inside).toHaveLength(LIMITS.partsInside);
    expect(addInside(line, "teeth")).toBe(line);
    expect(removeInside(addInside(mouth, "teeth"), 0)).toEqual(mouth);
    expect(removeInside(full, 0).inside).toHaveLength(LIMITS.partsInside - 1);
  });
});

describe("stages", () => {
  test("a stage is added as a copy after the one chosen and removed down to two, within the limits", () => {
    const added = addStage(doc(), 1);

    expect(added.mouths).toHaveLength(6);
    expect(added.mouths[2]).toEqual(DEFAULT_FACES.mouths[1]!);

    let fewer = doc();

    for (let i = 0; i < 10; i++) fewer = removeStage(fewer, 0);
    expect(fewer.mouths).toHaveLength(LIMITS.minStages);

    let more = doc();

    for (let i = 0; i < 10; i++) more = addStage(more, 0);
    expect(more.mouths).toHaveLength(LIMITS.maxStages);
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
    expect(changed(withParts(doc(), { tab: "mouths", stage: 0 }, [dot]), doc())).toBe(true);
  });

  test("a document the server would refuse is refused here first, with the same sentence", () => {
    expect(refusalOf(doc())).toBeNull();
    expect(refusalOf(removeStage({ ...doc(), mouths: [[], []] }, 0))).toBeNull();
    expect(refusalOf({ ...doc(), mouths: [[]] })).toBe("입 모양 단계는 2개에서 8개 사이여야 해요.");
  });
});
