// What the one select a value is chosen in searches, and when (docs/face.md, 「The screens' shared
// style」), for the standalone screens' copy of bmo-ui's ChoiceSelect: a list searches once it holds
// more than seven choices and never with seven or fewer, the typed phrase matched whole against what a
// choice shows, letter case folded, and the matched part marked where it stands. The console bundle
// takes the layer's own select in the copy's place, so this keeps the layer's rule and matching.

import type { SelectMenuItem } from "@nuxt/ui";

// A list holding more choices than this searches.
export const SEARCH_ABOVE = 7;

// One piece of a choice's words, saying whether the search found it there.
export interface ChoicePart {
  text: string;
  hit: boolean;
}

// A heading or a separator of a list, which stands among the choices and is none of them.
function structural(item: SelectMenuItem): boolean {
  return typeof item === "object" && item !== null && (item.type === "label" || item.type === "separator");
}

// How many choices a list holds: its headings and separators are none.
export function choiceCount(items: readonly SelectMenuItem[]): number {
  return items.filter((item) => !structural(item)).length;
}

// Whether a list searches.
export function searchesChoices(items: readonly SelectMenuItem[]): boolean {
  return choiceCount(items) > SEARCH_ABOVE;
}

// The words a choice shows: a plain value is its own label.
export function choiceLabel(item: SelectMenuItem, labelKey = "label"): string {
  if (typeof item !== "object" || item === null) {
    return item === undefined ? "" : String(item);
  }

  const label = (item as Record<string, unknown>)[labelKey];

  return typeof label === "string" ? label : "";
}

// The words of the choice holding the value, read from the whole list so a search that narrows the
// chosen choice away leaves the control naming it; none where no choice holds the value. A choice's
// value is its field under the value key, or the choice itself where no key is given.
export function chosenLabel(items: readonly SelectMenuItem[], value: unknown, valueKey?: string, labelKey = "label"): string | undefined {
  if (value === undefined || value === null) {
    return undefined;
  }

  const chosen = items.find((item) => {
    if (structural(item)) {
      return false;
    }

    const own = valueKey !== undefined && typeof item === "object" && item !== null ? (item as Record<string, unknown>)[valueKey] : item;

    return own === value;
  });

  return chosen === undefined ? undefined : choiceLabel(chosen, labelKey);
}

// Every place the phrase stands in the text, letter case folded, as [start, end) pairs in order.
function hits(text: string, words: string, locale: string | undefined): [number, number][] {
  const phrase = words.trim().toLocaleLowerCase(locale);

  if (phrase === "" || text === "") {
    return [];
  }

  const haystack = text.toLocaleLowerCase(locale);
  const found: [number, number][] = [];

  for (let from = haystack.indexOf(phrase); from !== -1; from = haystack.indexOf(phrase, from + phrase.length)) {
    found.push([from, from + phrase.length]);
  }

  return found;
}

// The choices whose words hold the phrase, in the order they came; every choice for no words. A
// heading and a separator stand among a narrowed list no more.
export function narrowChoices(items: readonly SelectMenuItem[], words: string, labelKey = "label", locale?: string): SelectMenuItem[] {
  if (words.trim() === "") {
    return [...items];
  }

  return items.filter((item) => !structural(item) && hits(choiceLabel(item, labelKey), words, locale).length > 0);
}

// A choice's words cut into the pieces the list draws, each saying whether it is a hit.
export function choiceParts(text: string, words: string, locale?: string): ChoicePart[] {
  const parts: ChoicePart[] = [];
  let at = 0;

  for (const [start, end] of hits(text, words, locale)) {
    if (start > at) parts.push({ text: text.slice(at, start), hit: false });
    parts.push({ text: text.slice(start, end), hit: true });
    at = end;
  }

  if (at < text.length) parts.push({ text: text.slice(at), hit: false });

  return parts;
}
