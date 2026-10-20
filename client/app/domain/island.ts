import type { Ref } from "vue";

// The one place these screens say anything to whoever is working in it: the word that an action went
// through or did not, and the question a press that cannot be taken back asks first. Both are drawn
// as one pill standing at the top of the screen (`MessageIsland`).
//
// What makes it a system rather than a component is the queue: two pieces of news arriving together
// do not stack into a column, because a column of them covers the screen it is reporting on. One
// stands, the rest wait, and the one standing says how many are behind it.

export type IslandTone = "info" | "success" | "warning" | "error";

// A press offered inside the island. A question's two answers are such presses.
export interface IslandAction {
  label: string;
  tone?: "neutral" | "primary" | "error";
  press: () => void | Promise<void>;
}

export interface IslandMessage {
  // What makes two notices the same notice said twice: the second replaces the first where it
  // stands instead of queueing behind itself.
  key: string;
  tone: IslandTone;
  icon: string;
  title: string;
  description: string | null;
  actions: IslandAction[];
  // How long it stands, in milliseconds. Null stands until it is answered.
  duration: number | null;
  // A question holds the island until it is answered: its words are not a press, and only its two
  // actions and the back gesture end it.
  question: boolean;
  // Whether the back gesture takes this question back. It does, except for a question asked while
  // the page is already being left: that question is holding a navigation, which moves the history
  // itself.
  back: boolean;
}

// What a caller writes. Everything but the words has a default, so the common case is one line.
export interface IslandSaid {
  key?: string;
  tone?: IslandTone;
  icon?: string;
  title: string;
  description?: string | null;
  duration?: number | null;
}

export interface IslandAsked {
  title: string;
  description?: string | null;
  // The word on the button that goes through with it. It names the action alone.
  confirm: string;
  cancel?: string;
  tone?: IslandTone;
  back?: boolean;
}

export interface IslandState {
  standing: IslandMessage | null;
  waiting: IslandMessage[];
}

export type Translate = (key: string, values?: Record<string, unknown>) => string;

export const TONE_ICONS: Record<IslandTone, string> = {
  info: "i-material-symbols-info-outline",
  success: "i-material-symbols-check-circle-outline",
  warning: "i-material-symbols-warning-outline",
  error: "i-material-symbols-error-outline"
};

// How long each tone stands. Something that went wrong is read more slowly than something that went
// through, and is given longer for it.
export const TONE_MS: Record<IslandTone, number> = {
  info: 4_000,
  success: 4_000,
  warning: 6_000,
  error: 6_000
};

// How many may wait behind the one standing. Past this the oldest of the waiting is dropped: what a
// reader wants out of a pile of news is the newest of it.
export const WAITING_MAX = 5;

export function emptyIsland(): IslandState {
  return { standing: null, waiting: [] };
}

let counted = 0;

// The words filled out into a message. A caller that named no key gets one of its own, so two
// separate sayings of the same words are two notices rather than one replacing the other.
export function messageOf(said: IslandSaid): IslandMessage {
  const tone = said.tone ?? "info";

  counted += 1;

  return {
    key: said.key ?? `island:${counted}`,
    tone,
    icon: said.icon ?? TONE_ICONS[tone],
    title: said.title,
    description: said.description ?? null,
    actions: [],
    duration: said.duration === undefined ? TONE_MS[tone] : said.duration,
    question: false,
    back: false
  };
}

export function questionOf(t: Translate, asked: IslandAsked, answer: (taken: boolean) => void): IslandMessage {
  const tone = asked.tone ?? "warning";

  counted += 1;

  return {
    key: `island:${counted}`,
    tone,
    icon: TONE_ICONS[tone],
    title: asked.title,
    description: asked.description ?? null,
    actions: [
      { label: asked.cancel ?? t("common.action.cancel"), tone: "neutral", press: () => answer(false) },
      { label: asked.confirm, tone: tone === "error" || tone === "warning" ? "error" : "primary", press: () => answer(true) }
    ],
    duration: null,
    question: true,
    back: asked.back ?? true
  };
}

// Whether the island widens into a card for this message rather than standing as one line.
export function expands(message: IslandMessage): boolean {
  return message.description !== null || message.actions.length > 0;
}

// Whether pressing the words does anything. A notice's words dismiss it; a question's are not a
// press, since only its answers end it.
export function pressable(message: IslandMessage): boolean {
  return !message.question;
}

function without(waiting: IslandMessage[], key: string): IslandMessage[] {
  return waiting.filter((one) => one.key !== key);
}

// The line held to its length, dropping its oldest end.
function capped(line: IslandMessage[]): IslandMessage[] {
  return line.slice(Math.max(0, line.length - WAITING_MAX));
}

// A new message arriving. The order the cases are tried in is the whole of the rule.
export function raise(state: IslandState, message: IslandMessage): IslandState {
  // The same thing said again, wherever it is: it replaces itself rather than queueing behind
  // itself, and where it was already standing its time starts over.
  if (state.standing?.key === message.key) {
    return { standing: message, waiting: state.waiting };
  }

  if (state.waiting.some((one) => one.key === message.key)) {
    return { standing: state.standing, waiting: state.waiting.map((one) => (one.key === message.key ? message : one)) };
  }

  if (state.standing === null) {
    return { standing: message, waiting: state.waiting };
  }

  // A question is asked by a press the reader has just made, so it takes the island at once and
  // what it displaced goes to the head of the line rather than to the back of it.
  if (message.question) {
    return { standing: message, waiting: capped([state.standing, ...state.waiting]) };
  }

  return { standing: state.standing, waiting: capped([...state.waiting, message]) };
}

// One message done with, however it ended. The next in line takes the island straight away, so the
// island morphs from one to the next rather than going away and coming back.
export function settle(state: IslandState, key: string): IslandState {
  if (state.standing?.key !== key) {
    return { standing: state.standing, waiting: without(state.waiting, key) };
  }

  const [next, ...rest] = state.waiting;

  return { standing: next ?? null, waiting: rest };
}

export interface Island {
  say: (said: IslandSaid) => string;
  ask: (asked: IslandAsked) => Promise<boolean>;
}

export const ISLAND_STATE = "bmo-face-island";

export function islandState(): Ref<IslandState> {
  return useState<IslandState>(ISLAND_STATE, emptyIsland);
}

export function useIsland(t: Translate): Island {
  const state = islandState();
  const put = (message: IslandMessage): string => {
    state.value = raise(state.value, message);

    return message.key;
  };

  return {
    say: (said) => put(messageOf(said)),
    ask: (asked) =>
      new Promise<boolean>((resolve) => {
        const message = questionOf(t, asked, (taken) => {
          state.value = settle(state.value, message.key);
          resolve(taken);
        });

        put(message);
      })
  };
}
