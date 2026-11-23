import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";

// The screens' language packs (docs/face.md, 「The console screen」): Korean, English, Japanese and Simplified
// Chinese, each holding every key the Korean one holds, in the same order, each a word with the same
// placeholders and markup.

const FOLDER = new URL("../i18n/locales/", import.meta.url);
const LOCALES = ["en", "ja", "ko", "zh-CN"];

type Tree = { [key: string]: string | Tree };

function leaves(tree: Tree, prefix = ""): Map<string, string> {
  return new Map(
    Object.entries(tree).flatMap(([key, value]) =>
      typeof value === "string" ? [[`${prefix}${key}`, value] as const] : [...leaves(value, `${prefix}${key}.`)],
    ),
  );
}

const packs = new Map(
  readdirSync(FOLDER)
    .filter((file) => file.endsWith(".yaml"))
    .map((file) => [file.slice(0, -".yaml".length), leaves(Bun.YAML.parse(readFileSync(new URL(file, FOLDER), "utf8")) as Tree)]),
);

const korean = packs.get("ko")!;

// The placeholders, linked messages and markup tags a word holds, in whatever order its language puts them.
const marks = (word: string): string[] => [...word.matchAll(/\{\w+\}|@:[\w.]+|<\/?\w+[^>]*>/g)].map((found) => found[0]).sort();

test("the screens speak Korean, English, Japanese and Simplified Chinese", () => {
  expect([...packs.keys()].sort()).toEqual(LOCALES);
});

test("every pack holds the Korean pack's keys, in the same order, each a word", () => {
  const keys = [...korean.keys()];

  for (const [locale, pack] of packs) {
    expect({ locale, keys: [...pack.keys()] }).toEqual({ locale, keys });
    for (const [key, word] of pack) expect({ locale, key, empty: word.trim() === "" }).toEqual({ locale, key, empty: false });
  }
});

test("every word carries the placeholders and markup the Korean one does", () => {
  for (const [locale, pack] of packs) {
    for (const [key, word] of pack) {
      expect({ locale, key, marks: marks(word) }).toEqual({ locale, key, marks: marks(korean.get(key)!) });
    }
  }
});
