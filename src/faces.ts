// The expression document: the one statement of what the display draws and the editor saves — its
// shape, the check every save goes through, and the default a new data root starts from
// (docs/face.md, 「The expression document」). The server checks with it and the screen imports it,
// so neither keeps a copy of its own.

/** The emotions BMO speaks a briefing with; one face each. None named reads as `기본`. */
export const EMOTIONS = ["기본", "기쁨", "슬픔", "놀람", "화남", "졸림", "폭소", "걱정", "아픔", "안간힘"] as const;
export type Emotion = (typeof EMOTIONS)[number];

/** What BMO is doing, as its presence names it; one face each. */
export const STATES = ["writing", "thinking", "idle"] as const;
export type FaceState = (typeof STATES)[number];

export const FACE_NAMES = [...EMOTIONS, ...STATES] as const;
export type FaceName = Emotion | FaceState;

/** The face every missing face falls back to, which a document must therefore hold. */
export const FALLBACK_FACE: FaceName = "기본";

/** The groups a face is drawn in, in drawing order, each over the ones before it. */
export const PART_GROUPS = ["eyes", "brows", "cheeks", "mouth", "marks"] as const;
export type PartGroup = (typeof PART_GROUPS)[number];

/** The paints a part is filled and lined with; each is a colour of the document. */
export const PAINTS = ["ink", "light", "mouth", "tongue", "cheek", "tear"] as const;
export type Paint = (typeof PAINTS)[number];

/** The screen's own coordinates: every anchor and every handle end stands inside this box. */
export const SCREEN = { width: 100, height: 75 } as const;

export const LIMITS = {
  partsPerGroup: 16,
  partsInside: 8,
  minNodes: 2,
  maxNodes: 32,
  minStages: 2,
  maxStages: 8,
  minWidth: 0.2,
  maxWidth: 10,
} as const;

export type Point = [number, number];

/**
 * One node of a path: the anchor it passes through and its two handles, each an offset from the
 * anchor — the one the path comes in by and the one it leaves by. A smooth node's handles stand on
 * one line through the anchor, which the editor keeps; the drawing reads only the handles.
 */
export interface PathNode {
  anchor: Point;
  coming: Point;
  leaving: Point;
  kind: "smooth" | "corner";
}

export interface PartLine {
  paint: Paint;
  width: number;
}

/** One path of cubic Bézier segments, with its own paints and, when closed, the parts cut off inside it. */
export interface Part {
  nodes: PathNode[];
  closed: boolean;
  fill: Paint | null;
  line: PartLine | null;
  inside?: Part[];
}

export type Face = Record<PartGroup, Part[]>;

export type FaceColors = { body: string; screen: string } & Record<Paint, string>;

export interface FaceDocument {
  colors: FaceColors;
  faces: Partial<Record<FaceName, Face>>;
  /** The mouths while speaking, from closed to widest open. */
  mouths: Part[][];
  /** When it was last saved, in epoch milliseconds; also its version. */
  savedAt: number;
}

export function isFaceName(word: unknown): word is FaceName {
  return typeof word === "string" && (FACE_NAMES as readonly string[]).includes(word);
}

/** The face drawn for a name: its own, or the fallback's where the document holds none. */
export function faceFor(document: FaceDocument, name: FaceName): Face {
  return document.faces[name] ?? (document.faces[FALLBACK_FACE] as Face);
}

// ---------------------------------------------------------------------------------------------
// Shapes built from nodes: the spline through points and the oval.

/**
 * Smooth nodes for the spline through `points`: each leaving handle a sixth of the step from the
 * point before to the point after, and the coming handle that step reversed. The end of an open run
 * stands for its own missing neighbour; a closed run's neighbours wrap round.
 */
export function splineNodes(points: Point[], closed: boolean): PathNode[] {
  const count = points.length;
  const at = (index: number): Point => points[closed ? (index + count) % count : Math.min(count - 1, Math.max(0, index))]!;

  return points.map(([x, y], index) => {
    const [px, py] = at(index - 1);
    const [nx, ny] = at(index + 1);

    return { anchor: [x, y], coming: [(px - nx) / 6, (py - ny) / 6], leaving: [(nx - px) / 6, (ny - py) / 6], kind: "smooth" };
  });
}

/** How far along a quarter of an ellipse its handles reach, as a share of the radius. */
export const OVAL_HANDLE = (4 / 3) * (Math.SQRT2 - 1);

/** Four smooth nodes at the ends of the radii, clockwise from the right, that draw the ellipse. */
export function ovalNodes(cx: number, cy: number, rx: number, ry: number): PathNode[] {
  const hx = rx * OVAL_HANDLE;
  const hy = ry * OVAL_HANDLE;

  return [
    { anchor: [cx + rx, cy], coming: [0, -hy], leaving: [0, hy], kind: "smooth" },
    { anchor: [cx, cy + ry], coming: [hx, 0], leaving: [-hx, 0], kind: "smooth" },
    { anchor: [cx - rx, cy], coming: [0, hy], leaving: [0, -hy], kind: "smooth" },
    { anchor: [cx, cy - ry], coming: [-hx, 0], leaving: [hx, 0], kind: "smooth" },
  ];
}

// ---------------------------------------------------------------------------------------------
// The check. Every refusal is one Korean sentence the editor can show as it stands.

type Checked<T> = { ok: T } | { error: string };

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);
const isNumber = (value: unknown): value is number => typeof value === "number" && Number.isFinite(value);
const isPaint = (value: unknown): value is Paint => typeof value === "string" && (PAINTS as readonly string[]).includes(value);
const COLOR = /^#[0-9a-f]{6}$/i;

/** Room for the last bit of floating point in a handle end computed from an anchor on the edge. */
const EDGE = 1e-9;

function within(x: number, y: number): boolean {
  return x >= -EDGE && x <= SCREEN.width + EDGE && y >= -EDGE && y <= SCREEN.height + EDGE;
}

function isPoint(value: unknown): value is Point {
  return Array.isArray(value) && value.length === 2 && isNumber(value[0]) && isNumber(value[1]);
}

function checkNode(value: unknown, where: string): Checked<PathNode> {
  if (!isRecord(value) || !isPoint(value.anchor) || !isPoint(value.coming) || !isPoint(value.leaving) || (value.kind !== "smooth" && value.kind !== "corner")) {
    return { error: `${where}: 노드의 값이 맞지 않아요.` };
  }

  const [x, y] = value.anchor;
  const ends = [value.anchor, [x + value.coming[0], y + value.coming[1]], [x + value.leaving[0], y + value.leaving[1]]];

  if (ends.some(([ex, ey]) => !within(ex!, ey!))) {
    return { error: `${where}: 노드나 손잡이가 화면 밖으로 나가요.` };
  }

  return { ok: { anchor: [x, y], coming: [value.coming[0], value.coming[1]], leaving: [value.leaving[0], value.leaving[1]], kind: value.kind } };
}

function checkLine(value: unknown, where: string): Checked<PartLine | null> {
  if (value === null) {
    return { ok: null };
  }

  if (!isRecord(value) || !isPaint(value.paint) || !isNumber(value.width)) {
    return { error: `${where}: 선의 값이 맞지 않아요.` };
  }

  if (value.width < LIMITS.minWidth || value.width > LIMITS.maxWidth) {
    return { error: `${where}: 선 굵기는 ${LIMITS.minWidth}에서 ${LIMITS.maxWidth} 사이여야 해요.` };
  }

  return { ok: { paint: value.paint, width: value.width } };
}

function checkPart(value: unknown, where: string, nested: boolean): Checked<Part> {
  if (!isRecord(value)) {
    return { error: `${where}: 모양이 아니에요.` };
  }

  const { nodes, closed, fill, inside } = value;

  if (typeof closed !== "boolean") {
    return { error: `${where}: 닫힌 선인지가 맞지 않아요.` };
  }

  if (!Array.isArray(nodes) || nodes.length < LIMITS.minNodes || nodes.length > LIMITS.maxNodes) {
    return { error: `${where}: 노드는 ${LIMITS.minNodes}개에서 ${LIMITS.maxNodes}개 사이여야 해요.` };
  }

  const checkedNodes: PathNode[] = [];

  for (const [index, item] of nodes.entries()) {
    const node = checkNode(item, `${where} 노드 ${index + 1}`);

    if ("error" in node) {
      return node;
    }

    checkedNodes.push(node.ok);
  }

  if (fill !== null && !isPaint(fill)) {
    return { error: `${where}: 채우기 칠이 맞지 않아요.` };
  }

  const line = checkLine(value.line, where);

  if ("error" in line) {
    return line;
  }

  if (fill === null && line.ok === null) {
    return { error: `${where}: 채우기나 선 가운데 하나는 있어야 해요.` };
  }

  if (fill !== null && !closed) {
    return { error: `${where}: 열린 선은 채울 수 없어요.` };
  }

  const part: Part = { nodes: checkedNodes, closed, fill, line: line.ok };

  if (inside === undefined) {
    return { ok: part };
  }

  if (nested) {
    return { error: `${where}: 안에 넣은 모양 안에는 다시 넣을 수 없어요.` };
  }

  if (!closed) {
    return { error: `${where}: 열린 선 안에는 모양을 넣을 수 없어요.` };
  }

  if (!Array.isArray(inside) || inside.length > LIMITS.partsInside) {
    return { error: `${where}: 안에 넣는 모양은 ${LIMITS.partsInside}개까지예요.` };
  }

  const checkedInside: Part[] = [];

  for (const [index, item] of inside.entries()) {
    const inner = checkPart(item, `${where} 안 ${index + 1}`, true);

    if ("error" in inner) {
      return inner;
    }

    checkedInside.push(inner.ok);
  }

  return { ok: checkedInside.length > 0 ? { ...part, inside: checkedInside } : part };
}

function checkParts(value: unknown, where: string): Checked<Part[]> {
  if (!Array.isArray(value) || value.length > LIMITS.partsPerGroup) {
    return { error: `${where}: 모양 목록이 맞지 않아요.` };
  }

  const parts: Part[] = [];

  for (const [index, item] of value.entries()) {
    const part = checkPart(item, `${where} ${index + 1}`, false);

    if ("error" in part) {
      return part;
    }

    parts.push(part.ok);
  }

  return { ok: parts };
}

function checkFace(value: unknown, name: string): Checked<Face> {
  if (!isRecord(value)) {
    return { error: `${name} 표정이 맞지 않아요.` };
  }

  const face = {} as Face;

  for (const group of PART_GROUPS) {
    const parts = checkParts(value[group] ?? [], `${name} ${group}`);

    if ("error" in parts) {
      return parts;
    }

    face[group] = parts.ok;
  }

  return { ok: face };
}

/** The document as it may be saved, or the one sentence saying why not. `savedAt` is not checked here. */
export function checkFaceDocument(value: unknown): Checked<Omit<FaceDocument, "savedAt">> {
  if (!isRecord(value) || !isRecord(value.colors) || !isRecord(value.faces) || !Array.isArray(value.mouths)) {
    return { error: "표정 문서의 모양이 맞지 않아요." };
  }

  const colors = {} as FaceColors;

  for (const key of ["body", "screen", ...PAINTS] as const) {
    const color = value.colors[key];

    if (typeof color !== "string" || !COLOR.test(color)) {
      return { error: `색 ${key}는 #rrggbb 꼴이어야 해요.` };
    }

    colors[key] = color.toLowerCase();
  }

  const faces: Partial<Record<FaceName, Face>> = {};

  for (const [name, face] of Object.entries(value.faces)) {
    if (!isFaceName(name)) {
      return { error: `모르는 표정 이름이에요: ${name}` };
    }

    const checked = checkFace(face, name);

    if ("error" in checked) {
      return checked;
    }

    faces[name] = checked.ok;
  }

  if (faces[FALLBACK_FACE] === undefined) {
    return { error: `${FALLBACK_FACE} 표정은 지울 수 없어요.` };
  }

  if (value.mouths.length < LIMITS.minStages || value.mouths.length > LIMITS.maxStages) {
    return { error: `입 모양 단계는 ${LIMITS.minStages}개에서 ${LIMITS.maxStages}개 사이여야 해요.` };
  }

  const mouths: Part[][] = [];

  for (const [index, stage] of value.mouths.entries()) {
    const parts = checkParts(stage, `입 모양 ${index + 1}단계`);

    if ("error" in parts) {
      return parts;
    }

    mouths.push(parts.ok);
  }

  return { ok: { colors, faces, mouths } };
}

// ---------------------------------------------------------------------------------------------
// The default: BMO's face as its finished official design draws it, measured from the reference
// pictures (docs/expressions.md, 「The common design」 and 「What the default draws」), on its pale mint
// screen, which a new data root starts from.

const node = (anchor: Point, coming: Point = [0, 0], leaving: Point = [0, 0], kind: PathNode["kind"] = "corner"): PathNode => ({ anchor, coming, leaving, kind });
const ink = (width: number) => ({ paint: "ink" as const, width });

/** Where the eyes stand: a quarter of the way in from each side, a third of the way down. */
const EYES = { left: 23.5, right: 76.5, y: 25 } as const;

/** A filled ink oval: a dot eye. */
const dot = (cx: number, cy: number, rx: number, ry: number): Part => ({ nodes: ovalNodes(cx, cy, rx, ry), closed: true, fill: "ink", line: null });

/** An open ink stroke through its points: an arc, a brow, a closed mouth. */
const line = (points: Point[], width = 1.8): Part => ({ nodes: splineNodes(points, false), closed: false, fill: null, line: ink(width) });

/** The two dot eyes, a little taller than wide, each moved by the same step from its place. */
const dots = (rx = 3.2, ry = 3.9, dx = 0, dy = 0): Part[] => [dot(EYES.left + dx, EYES.y + dy, rx, ry), dot(EYES.right + dx, EYES.y + dy, rx, ry)];

/** The two eyes as strokes: `shape` gives one eye's points around its centre, mirrored for the right. */
const strokeEyes = (shape: Point[], width = 1.8): Part[] => [
  line(shape.map(([x, y]): Point => [EYES.left + x, EYES.y + y]), width),
  line(shape.map(([x, y]): Point => [EYES.right - x, EYES.y + y]).reverse(), width),
];

/** The two brows: `shape` gives the left brow's points, mirrored for the right. */
const brows = (shape: Point[], width = 1.8): Part[] => [line(shape, width), line(shape.map(([x, y]): Point => [100 - x, y]).reverse(), width)];

/**
 * The upper teeth: one white band across the top of an opening and past its sides, cut off by the
 * outline, ending below in an edge that sags a little toward the middle.
 */
const teeth = (left: number, right: number, top: number, bottom: number, sag = 0.8): Part => {
  const pull = (right - left) / 4;

  return {
    nodes: [node([left, top]), node([right, top]), node([right, bottom], [0, 0], [-pull, sag]), node([left, bottom], [pull, sag])],
    closed: true,
    fill: "light",
    line: null,
  };
};

/** The lower teeth: one white band across the bottom of an opening, its upper edge rising a little toward the middle. */
const lowerTeeth = (left: number, right: number, top: number, bottom: number, rise = 0.5): Part => {
  const pull = (right - left) / 4;

  return {
    nodes: [node([left, top], [0, 0], [pull, -rise]), node([right, top], [-pull, -rise]), node([right, bottom]), node([left, bottom])],
    closed: true,
    fill: "light",
    line: null,
  };
};

/** The tongue: a dome rising from below an opening's bottom edge to its peak. */
const tongue = (left: number, right: number, base: number, peak: number): Part => {
  const rise = (peak - base) * 0.55;
  const reach = ((right - left) / 2) * 0.55;

  return {
    nodes: [node([left, base], [0, 0], [0, rise]), node([(left + right) / 2, peak], [-reach, 0], [reach, 0], "smooth"), node([right, base], [0, rise])],
    closed: true,
    fill: "tongue",
    line: null,
  };
};

/** An open mouth: its opening filled dark green and outlined in ink, with what shows inside it. */
const opening = (nodes: PathNode[], inside: Part[]): Part => ({ nodes, closed: true, fill: "mouth", line: ink(1.6), inside });

/**
 * BMO's smiling mouth, `width` wide and `height` high, centred between the eyes with its top at `top`:
 * a bowl whose top edge sags gently toward the middle and meets the round bottom in two corners,
 * with the upper teeth band along the top edge, thickest in the middle, and the tongue rising from
 * the bottom, its peak right of the middle, to a little above half the mouth's height.
 */
const smile = (width: number, height: number, top: number): Part => {
  const left = 50 - width / 2;
  const right = 50 + width / 2;
  const bottom = top + height;
  const sag = height * 0.1;
  const edge = width * 0.2;
  const drop = height * 0.55;
  const reach = width * 0.33;

  return opening(
    [
      node([left, top], [0, drop], [edge, sag * 0.6]),
      node([50, top + sag], [-edge, 0], [edge, 0], "smooth"),
      node([right, top], [-edge, sag * 0.6], [0, drop]),
      node([50, bottom], [reach, 0], [-reach, 0], "smooth"),
    ],
    [teeth(left - 1, right + 1, top - 1, top + height * 0.14, height * 0.24), tongue(left + width * 0.2, left + width * 1.02, bottom + 1, top + height * 0.42)],
  );
};

/**
 * An open wail, `width` wide and `height` high, centred between the eyes with its top at `top`: an
 * arched top and a flatter bottom, the teeth band along the arch and the tongue along the bottom.
 */
const wail = (width: number, height: number, top: number): Part => {
  const left = 50 - width / 2;
  const right = 50 + width / 2;
  const bottom = top + height;
  const corner = bottom - height * 0.2;
  const reach = width * 0.3;

  return opening(
    [
      node([left, corner], [width * 0.08, height * 0.2], [0, -height * 0.45]),
      node([50, top], [-reach, 0], [reach, 0], "smooth"),
      node([right, corner], [0, -height * 0.45], [-width * 0.08, height * 0.2]),
      node([50, bottom], [width * 0.25, 0], [-width * 0.25, 0], "smooth"),
    ],
    [teeth(left - 1, right + 1, top - 1, top + height * 0.3, height * 0.1), tongue(left + width * 0.15, right - width * 0.1, bottom + 1, bottom - height * 0.4)],
  );
};

/** An `x` eye: two straight strokes crossing at its centre. */
const crossEye = (cx: number, cy: number, reach = 3.2): Part[] => [
  line([[cx - reach, cy - reach], [cx + reach, cy + reach]], 2),
  line([[cx + reach, cy - reach], [cx - reach, cy + reach]], 2),
];

/** The calm mouth: one thin arc, 20 wide and 4 deep, its ends 12 below the eyes. */
const calm = (): Part => line([[40, EYES.y + 12], [50, EYES.y + 16], [60, EYES.y + 12]]);

const none = (): Pick<Face, "brows" | "cheeks" | "marks"> => ({ brows: [], cheeks: [], marks: [] });

export const DEFAULT_FACES: FaceDocument = {
  colors: {
    body: "#5fbb9c",
    screen: "#cef7d8",
    ink: "#000a00",
    light: "#fcfcfc",
    mouth: "#417640",
    tongue: "#66b567",
    cheek: "#f4a3b4",
    tear: "#74f1f0",
  },
  faces: {
    기본: { ...none(), eyes: dots(), mouth: [calm()] },
    기쁨: { ...none(), eyes: dots(), mouth: [smile(32, 15.5, 35)] },
    슬픔: {
      ...none(),
      eyes: strokeEyes([[-5.5, 2.5], [-1.5, -0.8], [5, -0.5]]),
      mouth: [wail(20, 9, 37)],
    },
    놀람: {
      ...none(),
      eyes: dots(3.6, 4.6),
      mouth: [opening(ovalNodes(50, 42, 6, 7), [teeth(43, 57, 34, 37.6, 0.4), tongue(44, 57, 50, 45.2)])],
    },
    화남: {
      ...none(),
      eyes: dots(),
      brows: brows([[17, 15.5], [29.5, 19.5]], 2.6),
      mouth: [wail(26, 11, 36)],
    },
    졸림: { ...none(), eyes: strokeEyes([[-4, 0], [0, 2.4], [4, 0]]), mouth: [line([[45.5, 39.5], [54.5, 39.5]])] },
    폭소: { ...none(), eyes: strokeEyes([[-4.5, 2], [0, -2], [4.5, 2]], 2), mouth: [smile(40, 18, 34.5)] },
    걱정: {
      ...none(),
      eyes: dots(),
      brows: brows([[18.5, 18.5], [28.5, 15.5]]),
      mouth: [
        {
          nodes: [node([41, 38.6], [0, 0], [4, -0.8]), node([59, 37.8], [-4, -0.4]), node([59, 41.6], [0, 0], [-4, 0.4]), node([41, 42.4], [4, 0.8])],
          closed: true,
          fill: "light",
          line: ink(1.4),
        },
      ],
    },
    아픔: {
      ...none(),
      eyes: [...crossEye(EYES.left, EYES.y), ...crossEye(EYES.right, EYES.y)],
      mouth: [opening([node([37, 37], [6.5, 8.5], [7.5, 3]), node([63, 37], [-7.5, 3], [-6.5, 8.5])], [teeth(34, 66, 34, 40.8, 0.6)])],
    },
    안간힘: {
      ...none(),
      eyes: dots(3.4, 2.2),
      mouth: [
        opening(
          [node([35, 42]), node([50, 38], [-8, 0], [8, 0], "smooth"), node([65, 42]), node([50, 46], [8, 0], [-8, 0], "smooth")],
          [teeth(33, 67, 36, 41.6, 0.3), lowerTeeth(33, 67, 42.4, 48, 0.3)],
        ),
      ],
    },
    writing: {
      ...none(),
      eyes: dots(3.2, 3.4, 0, 2),
      brows: brows([[19, 18.5], [28, 18.5]], 1.6),
      mouth: [line([[46, 40], [54, 40]])],
    },
    thinking: {
      ...none(),
      eyes: dots(3.2, 3.9, -2, -2),
      brows: [line([[17.5, 16], [21.5, 13.5], [25.5, 15]], 1.6)],
      mouth: [line([[44, 40.5], [51, 40], [57, 37.5]])],
    },
    idle: { ...none(), eyes: dots(), mouth: [line([[45, 39], [55, 39]])] },
  },
  mouths: [[calm()], [smile(16, 6, 36.5)], [smile(22, 9.5, 36)], [smile(27, 12.5, 35.5)], [smile(32, 15.5, 35)]],
  savedAt: 0,
};
