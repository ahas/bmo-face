import { closeTouchTooltip } from "./tooltips";

// The interface's one long press: a finger resting on one point, opened once it has stayed there.
// What it opens is the claim it made on the press, and the only thing a screen supplies. It is
// cancelled the moment the finger slides beyond the platform's touch distance from where it landed,
// the page under it scrolls, a second finger lands or the finger lifts, so it is never mistaken for
// a scroll or a drag. Once fired, what it opened stays, and the lift presses nothing.
//
// Every long press of the screens goes through this module. A component says what a press on it
// is for and gets the rest, so no screen holds a timer of its own.

export const PRESS_DELAY_MS = 500;
export const PRESS_SLOP = 10;

export interface Point {
  x: number;
  y: number;
}

export interface PressClaim {
  /** What the press stands on. The click the lifted finger makes inside it goes nowhere. */
  target: Element;
  /** Opens what the press is for. Answers whether anything opened. */
  fire: () => boolean | void;
}

export type PressClaimer = (event: PointerEvent) => PressClaim | null;

// The wait a press stands for, as the page measures time.
export type PressWait = (fire: () => void, ms: number) => () => void;

// How far a finger may slide before the press is no longer the same press.
export function strayed(from: Point, to: Point, slop = PRESS_SLOP): boolean {
  return Math.hypot(to.x - from.x, to.y - from.y) > slop;
}

export interface PressGauge {
  readonly standing: boolean;
  down(id: number, at: Point): boolean;
  move(id: number, at: Point): void;
  up(id: number): void;
  cancel(): void;
}

// One press at a time, apart from the page: started at a point, ended by the wait or by the events
// that take it back. A second finger starts nothing; it ends what stands.
export function pressGauge(wait: PressWait, fire: () => void, slop = PRESS_SLOP): PressGauge {
  let held: { id: number; from: Point } | null = null;
  let stop: (() => void) | null = null;

  function release(): void {
    stop?.();
    stop = null;
    held = null;
  }

  return {
    get standing(): boolean {
      return held !== null;
    },
    down(id: number, at: Point): boolean {
      if (held !== null) {
        // A second finger ends the press that stands rather than beginning another.
        release();

        return false;
      }

      held = { id, from: at };
      stop = wait(() => {
        stop = null;
        held = null;
        fire();
      }, PRESS_DELAY_MS);

      return true;
    },
    move(id: number, at: Point): void {
      if (held !== null && held.id === id && strayed(held.from, at, slop)) {
        release();
      }
    },
    up(id: number): void {
      if (held !== null && held.id === id) {
        release();
      }
    },
    cancel: release,
  };
}

// What the press of the moment stands on, and what the press that just ended stood on. A click
// inside either is read as the lift rather than as a press of its own.
let standing: PressClaim | null = null;
let fired: PressClaim | null = null;

// The pointer whose touch put a tooltip away. That touch has been spent and opens nothing, and
// the click its lift makes goes nowhere.
let spent: number | null = null;
let spentClick = false;

const claims: PressClaimer[] = [];
let listening = false;
let gauge: PressGauge | null = null;

function firstClaim(event: PointerEvent): PressClaim | null {
  for (const claim of claims) {
    const made = claim(event);

    if (made !== null) {
      return made;
    }
  }

  return null;
}

// Whether the touch of this pointer has already been spent on putting a tooltip away.
export function pressSpent(id: number): boolean {
  return spent === id;
}

function listen(): void {
  if (listening || import.meta.server) {
    return;
  }

  listening = true;

  gauge = pressGauge(
    (fire, ms) => {
      const timer = setTimeout(fire, ms);

      return () => clearTimeout(timer);
    },
    () => {
      const claim = standing;

      standing = null;

      if (claim === null || claim.fire() === false) {
        return;
      }

      // What was opened is the press's, and the click the lift makes goes nowhere.
      fired = claim;
    },
  );

  window.addEventListener(
    "pointerdown",
    (event) => {
      if (event.pointerType !== "touch" || gauge === null) {
        return;
      }

      standing = null;
      fired = null;
      spent = null;
      spentClick = false;

      // The touch that puts a tooltip away is spent on it: it opens nothing, it acts on nothing it
      // landed on, and it starts no press.
      if (closeTouchTooltip()) {
        spent = event.pointerId;
        spentClick = true;
        event.stopPropagation();

        return;
      }

      // A second finger ends the press rather than beginning another.
      if (gauge.standing) {
        gauge.cancel();

        return;
      }

      const claim = firstClaim(event);

      if (claim === null) {
        return;
      }

      standing = claim;
      gauge.down(event.pointerId, { x: event.clientX, y: event.clientY });
    },
    true,
  );

  window.addEventListener(
    "pointermove",
    (event) => {
      gauge?.move(event.pointerId, { x: event.clientX, y: event.clientY });
    },
    true,
  );

  const end = (event: PointerEvent): void => {
    gauge?.up(event.pointerId);
    standing = null;
  };

  window.addEventListener("pointerup", end, true);
  window.addEventListener("pointercancel", end, true);

  // A page that starts to scroll under the finger has taken the press for the scroll.
  window.addEventListener(
    "scroll",
    () => {
      gauge?.cancel();
      standing = null;
    },
    true,
  );

  window.addEventListener(
    "click",
    (event) => {
      // The lift of the touch that put a tooltip away acts on nothing either.
      if (spentClick && event.detail !== 0) {
        spentClick = false;
        event.preventDefault();
        event.stopImmediatePropagation();

        return;
      }

      if (fired === null || !(event.target instanceof Node) || !fired.target.contains(event.target)) {
        return;
      }

      fired = null;
      event.preventDefault();
      event.stopImmediatePropagation();
    },
    true,
  );

  // A held finger raises the browser's own menu and selection over what it holds. On what a claim
  // stands on they are this press's, and are put away with it.
  for (const kind of ["contextmenu", "selectstart"] as const) {
    window.addEventListener(
      kind,
      (event) => {
        const claim = standing ?? fired;

        if (claim !== null && event.target instanceof Node && claim.target.contains(event.target)) {
          event.preventDefault();
          event.stopImmediatePropagation();
        }
      },
      true,
    );
  }
}

// Registers what a long press is for, and returns what unregisters it. A claimer is asked on every
// touch that lands, and the first that answers takes that press.
export function claimPress(claimer: PressClaimer): () => void {
  claims.push(claimer);
  listen();

  return () => {
    const at = claims.indexOf(claimer);

    if (at !== -1) {
      claims.splice(at, 1);
    }
  };
}
