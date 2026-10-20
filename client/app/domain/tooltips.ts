// The tooltip a held finger has opened. Only one stands at a time, and the touch that comes next
// puts it away without opening anything else; a tooltip that closes itself or is replaced leaves
// nothing behind here.

let standing: (() => void) | null = null;

export function holdTouchTooltip(close: () => void): void {
  if (standing !== null && standing !== close) {
    standing();
  }

  standing = close;
}

export function releaseTouchTooltip(close: () => void): void {
  if (standing === close) {
    standing = null;
  }
}

// Puts away the tooltip a held finger opened, if one stands. Answers whether it did.
export function closeTouchTooltip(): boolean {
  const close = standing;

  standing = null;

  if (close === null) {
    return false;
  }

  close();

  return true;
}
