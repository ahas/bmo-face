// How a number reads when it says how many or how much: with the thousands mark, in a locale
// written down rather than left to the machine, so every reader sees the same figure.
export const COUNT_LOCALE = "ko-KR";

export function countText(value: number): string {
  return value.toLocaleString(COUNT_LOCALE);
}

// How far a piece of work with a known end has come: done over total and the share, with no words
// around it. The share is floored, so 100% stands only once everything is done.
export function progressText(done: number, total: number): string {
  const share = total <= 0 ? 0 : Math.floor((Math.min(done, total) / total) * 100);

  return `${countText(done)}/${countText(total)} ${share}%`;
}
