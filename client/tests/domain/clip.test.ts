import { expect, test } from "bun:test";

import { isCut, visiblePart } from "../../app/domain/clip";

// Whether a line is cut to fit its box, and what part of it the reader can see (docs/face.md, 「The
// screens' shared style」). A box the text only meets is not cut.

const box = { left: 0, top: 0, right: 100, bottom: 20 };

test("text that runs past the right or the bottom is cut", () => {
  expect(isCut({ text: { left: 0, top: 0, right: 102, bottom: 20 }, box })).toBe(true);
  expect(isCut({ text: { left: 0, top: 0, right: 100, bottom: 41 }, box })).toBe(true);
});

test("text that meets its box is not cut", () => {
  expect(isCut({ text: { left: 0, top: 0, right: 100, bottom: 20 }, box })).toBe(false);
  expect(isCut({ text: { left: 0.5, top: 0, right: 99.5, bottom: 19 }, box })).toBe(false);
});

test("a whole line shifted past the start of its box is cut", () => {
  expect(isCut({ text: { left: -20, top: 0, right: 40, bottom: 20 }, box })).toBe(true);
});

test("the visible part is where the text and its box meet", () => {
  expect(visiblePart({ text: { left: 0, top: 0, right: 140, bottom: 20 }, box })).toEqual({ left: 0, top: 0, right: 100, bottom: 20 });
  expect(visiblePart({ text: { left: 0, top: 0, right: 100, bottom: 40 }, box })).toEqual({ left: 0, top: 0, right: 100, bottom: 20 });
});

test("a line wholly outside its box shows nothing", () => {
  expect(visiblePart({ text: { left: 120, top: 0, right: 200, bottom: 20 }, box })).toBeNull();
});
