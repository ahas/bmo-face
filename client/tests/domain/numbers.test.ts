import { expect, test } from "bun:test";

import { countText, progressText } from "../../app/domain/numbers";

test("a count carries the thousands mark whatever its size", () => {
  expect(countText(0)).toBe("0");
  expect(countText(999)).toBe("999");
  expect(countText(1631)).toBe("1,631");
  expect(countText(116002)).toBe("116,002");
});

test("progress reads done over total and a floored share, with no words", () => {
  expect(progressText(0, 1631)).toBe("0/1,631 0%");
  expect(progressText(1630, 1631)).toBe("1,630/1,631 99%");
  expect(progressText(1631, 1631)).toBe("1,631/1,631 100%");
  expect(progressText(2000, 1631)).toBe("2,000/1,631 100%");
  expect(progressText(0, 0)).toBe("0/0 0%");
});
