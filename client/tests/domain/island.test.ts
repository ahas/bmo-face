import { expect, test } from "bun:test";

import { WAITING_MAX, emptyIsland, expands, messageOf, pressable, questionOf, raise, settle, type IslandState } from "../../app/domain/island";

const t = (key: string) => key;
const said = (title: string, key?: string) => messageOf({ title, key });

test("the first message stands and the next waits behind it", () => {
  const first = said("first");
  const second = said("second");
  const state = raise(raise(emptyIsland(), first), second);

  expect(state.standing?.title).toBe("first");
  expect(state.waiting.map((one) => one.title)).toEqual(["second"]);
});

test("the same key said again replaces itself where it stands or waits", () => {
  let state = raise(raise(emptyIsland(), said("a", "k1")), said("b", "k2"));

  state = raise(state, said("a again", "k1"));
  expect(state.standing?.title).toBe("a again");

  state = raise(state, said("b again", "k2"));
  expect(state.waiting.map((one) => one.title)).toEqual(["b again"]);
});

test("a question takes the island at once and puts the notice it displaced first in line", () => {
  const notice = said("notice");
  const waiting = said("waiting");
  const question = questionOf(t, { title: "delete?", confirm: "delete" }, () => {});
  const state = raise(raise(raise(emptyIsland(), notice), waiting), question);

  expect(state.standing).toBe(question);
  expect(state.waiting.map((one) => one.title)).toEqual(["notice", "waiting"]);
});

test("the line is held to its length by dropping its oldest end", () => {
  let state: IslandState = raise(emptyIsland(), said("standing"));

  for (let n = 0; n < WAITING_MAX + 2; n += 1) {
    state = raise(state, said(`w${n}`));
  }

  expect(state.waiting.length).toBe(WAITING_MAX);
  expect(state.waiting[0]?.title).toBe("w2");
});

test("settling the one standing moves the next up, and settling a waiting one only removes it", () => {
  const a = said("a");
  const b = said("b");
  const c = said("c");
  let state = raise(raise(raise(emptyIsland(), a), b), c);

  state = settle(state, c.key);
  expect(state.standing).toBe(a);
  expect(state.waiting).toEqual([b]);

  state = settle(state, a.key);
  expect(state.standing).toBe(b);
  expect(state.waiting).toEqual([]);

  expect(settle(state, b.key).standing).toBeNull();
});

test("a question offers cancel then confirm, stands until answered, and answers through its actions", () => {
  const answers: boolean[] = [];
  const question = questionOf(t, { title: "leave?", confirm: "leave", back: false }, (taken) => answers.push(taken));

  expect(question.actions.map((one) => one.label)).toEqual(["common.action.cancel", "leave"]);
  expect(question.duration).toBeNull();
  expect(question.back).toBe(false);
  expect(pressable(question)).toBe(false);
  expect(expands(question)).toBe(true);

  question.actions[0]?.press();
  question.actions[1]?.press();
  expect(answers).toEqual([false, true]);
});

test("a notice times itself by its tone and is dismissed by a press", () => {
  const notice = messageOf({ title: "saved", tone: "success" });
  const failure = messageOf({ title: "failed", tone: "error" });

  expect(notice.duration).toBe(4_000);
  expect(failure.duration).toBe(6_000);
  expect(pressable(notice)).toBe(true);
  expect(expands(notice)).toBe(false);
  expect(expands(messageOf({ title: "x", description: "more" }))).toBe(true);
});
