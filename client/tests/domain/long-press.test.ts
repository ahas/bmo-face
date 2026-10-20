import { expect, test } from "bun:test";

import { PRESS_DELAY_MS, PRESS_SLOP, pressGauge, strayed } from "../../app/domain/long-press";

// The interface's one long press (docs/face.md, 「The screens' shared style」): a finger that stays
// opens what it holds, and every way the platform ends a press without one ends it here too.

function harness() {
  let waiting: (() => void) | null = null;
  let fired = 0;

  const gauge = pressGauge(
    (fire, ms) => {
      expect(ms).toBe(PRESS_DELAY_MS);
      waiting = () => {
        waiting = null;
        fire();
      };

      return () => {
        waiting = null;
      };
    },
    () => {
      fired += 1;
    },
  );

  return {
    gauge,
    fire: () => waiting?.(),
    get fired() {
      return fired;
    },
  };
}

test("a finger that stays opens what it holds, once", () => {
  const press = harness();

  expect(press.gauge.down(1, { x: 0, y: 0 })).toBe(true);
  expect(press.gauge.standing).toBe(true);

  press.fire();
  press.fire();

  expect(press.fired).toBe(1);
  expect(press.gauge.standing).toBe(false);
});

test("a finger lifted before the wait opens nothing", () => {
  const press = harness();

  press.gauge.down(1, { x: 0, y: 0 });
  press.gauge.up(1);
  press.fire();

  expect(press.fired).toBe(0);
  expect(press.gauge.standing).toBe(false);
});

test("a finger that slides past the touch distance ends the press", () => {
  const press = harness();

  press.gauge.down(1, { x: 20, y: 20 });
  press.gauge.move(1, { x: 20 + PRESS_SLOP + 1, y: 20 });
  press.fire();

  expect(press.gauge.standing).toBe(false);
  expect(press.fired).toBe(0);
});

test("a finger that stays within the touch distance does not end the press", () => {
  const press = harness();

  press.gauge.down(1, { x: 20, y: 20 });
  press.gauge.move(1, { x: 20 + PRESS_SLOP, y: 20 });
  press.fire();

  expect(press.fired).toBe(1);
});

test("a scroll and a second finger both end the press", () => {
  const scrolled = harness();

  scrolled.gauge.down(1, { x: 0, y: 0 });
  scrolled.gauge.cancel();
  scrolled.fire();

  const second = harness();

  second.gauge.down(1, { x: 0, y: 0 });
  expect(second.gauge.down(2, { x: 40, y: 40 })).toBe(false);
  second.fire();

  expect(scrolled.fired).toBe(0);
  expect(second.fired).toBe(0);
});

test("another finger moving does not end the press", () => {
  const press = harness();

  press.gauge.down(1, { x: 0, y: 0 });
  press.gauge.move(2, { x: 80, y: 80 });
  press.fire();

  expect(press.fired).toBe(1);
});

test("the distance is the platform's own", () => {
  expect(strayed({ x: 0, y: 0 }, { x: PRESS_SLOP, y: 0 })).toBe(false);
  expect(strayed({ x: 0, y: 0 }, { x: PRESS_SLOP + 0.5, y: 0 })).toBe(true);
  expect(strayed({ x: 0, y: 0 }, { x: 6, y: 8 })).toBe(false);
  expect(strayed({ x: 0, y: 0 }, { x: 7, y: 8 })).toBe(true);
});
