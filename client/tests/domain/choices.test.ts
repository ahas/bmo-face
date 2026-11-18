import { expect, test } from "bun:test";

import { choiceParts, chosenLabel, narrowChoices, searchesChoices, SEARCH_ABOVE } from "../../app/domain/choices";

// The standalone screens' select keeps the layer's rule (docs/face.md, 「The screens' shared style」).

const choices = (count: number) => Array.from({ length: count }, (_, index) => ({ label: `Choice ${index + 1}`, value: index }));

test("a list searches above seven choices and never at seven or fewer", () => {
  expect(SEARCH_ABOVE).toBe(7);
  expect(searchesChoices(choices(2))).toBe(false);
  expect(searchesChoices(choices(7))).toBe(false);
  expect(searchesChoices(choices(8))).toBe(true);
  expect(searchesChoices(choices(32))).toBe(true);
});

test("a heading and a separator are counted as no choice", () => {
  const items = [{ type: "label" as const, label: "Group" }, ...choices(7), { type: "separator" as const }];

  expect(searchesChoices(items)).toBe(false);
});

test("a list is narrowed to the labels holding the typed phrase, letter case folded", () => {
  const items = [{ label: "Happy", value: "happy" }, { label: "Sad", value: "sad" }, { label: "unhappy", value: "unhappy" }, "HAPPENING"];

  expect(narrowChoices(items, "  ")).toEqual(items);
  expect(narrowChoices(items, "hap")).toEqual([items[0], items[2], "HAPPENING"]);
  expect(narrowChoices(items, "happy u")).toEqual([]);
  expect(narrowChoices([{ type: "label", label: "Happy" }, ...items], "happy")).toEqual([items[0], items[2]]);
});

test("a label is cut where the phrase stands, every place it stands", () => {
  expect(choiceParts("Node 12", "")).toEqual([{ text: "Node 12", hit: false }]);
  expect(choiceParts("Node 12", "node")).toEqual([
    { text: "Node", hit: true },
    { text: " 12", hit: false },
  ]);
  expect(choiceParts("abab", "B")).toEqual([
    { text: "a", hit: false },
    { text: "b", hit: true },
    { text: "a", hit: false },
    { text: "b", hit: true },
  ]);
});

test("the chosen choice is named from the whole list, whatever the search narrowed it to", () => {
  const items = [{ type: "label" as const, label: "Group" }, { label: "Happy", value: "happy" }, { label: "Sad", value: 0 }, "Plain"];

  expect(chosenLabel(items, "happy", "value")).toBe("Happy");
  expect(chosenLabel(items, 0, "value")).toBe("Sad");
  expect(chosenLabel(items, "Plain")).toBe("Plain");
  expect(chosenLabel(items, "missing", "value")).toBeUndefined();
  expect(chosenLabel(items, undefined, "value")).toBeUndefined();
});
