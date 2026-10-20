// What every list dropped from a control stands as: as wide as its longest item and never narrower
// than the control it drops from, but at most the screen less a gutter each side, its items
// scrolling sideways together past that. The floor is named whole, class and all, since the
// stylesheet is built from the words written here.
function dropdown(floor: string) {
  return {
    content: `w-max max-w-[calc(100vw-1rem)] ${floor}`,
    viewport: "overflow-x-auto",
    group: "w-max min-w-full",
    itemLabel: "text-clip whitespace-nowrap",
    itemDescription: "text-clip whitespace-nowrap"
  };
}

export default defineAppConfig({
  ui: {
    // The marks Nuxt UI draws on its own components, in the one icon set these screens use.
    icons: {
      arrowDown: "i-material-symbols-arrow-downward",
      arrowLeft: "i-material-symbols-arrow-back",
      arrowRight: "i-material-symbols-arrow-forward",
      arrowUp: "i-material-symbols-arrow-upward",
      caution: "i-material-symbols-error-outline",
      check: "i-material-symbols-check",
      chevronDoubleLeft: "i-material-symbols-keyboard-double-arrow-left",
      chevronDoubleRight: "i-material-symbols-keyboard-double-arrow-right",
      chevronDown: "i-material-symbols-keyboard-arrow-down",
      chevronLeft: "i-material-symbols-chevron-left",
      chevronRight: "i-material-symbols-chevron-right",
      chevronUp: "i-material-symbols-keyboard-arrow-up",
      close: "i-material-symbols-close",
      copy: "i-material-symbols-content-copy-outline",
      copyCheck: "i-material-symbols-library-add-check-outline",
      dark: "i-material-symbols-dark-mode-outline",
      drag: "i-material-symbols-drag-indicator",
      ellipsis: "i-material-symbols-more-horiz",
      error: "i-material-symbols-cancel-outline",
      external: "i-material-symbols-arrow-outward",
      eye: "i-material-symbols-visibility-outline",
      eyeOff: "i-material-symbols-visibility-off-outline",
      file: "i-material-symbols-draft-outline",
      folder: "i-material-symbols-folder-outline",
      folderOpen: "i-material-symbols-folder-open-outline",
      hash: "i-material-symbols-tag",
      info: "i-material-symbols-info-outline",
      light: "i-material-symbols-light-mode-outline",
      loading: "i-material-symbols-progress-activity",
      menu: "i-material-symbols-menu",
      minus: "i-material-symbols-remove",
      panelClose: "i-material-symbols-left-panel-close-outline",
      panelOpen: "i-material-symbols-left-panel-open-outline",
      plus: "i-material-symbols-add",
      reload: "i-material-symbols-refresh",
      search: "i-material-symbols-search",
      stop: "i-material-symbols-stop-outline",
      star: "i-material-symbols-star-outline",
      success: "i-material-symbols-check-circle-outline",
      system: "i-material-symbols-desktop-windows-outline",
      tip: "i-material-symbols-lightbulb-outline",
      upload: "i-material-symbols-upload",
      warning: "i-material-symbols-warning-outline"
    },
    colors: {
      primary: "teal",
      neutral: "zinc"
    },
    // A place waiting on its content does not pulse for a reader who asked for less movement.
    skeleton: {
      base: "animate-pulse rounded-md bg-elevated motion-reduce:animate-none"
    },
    // An icon-only button is the square the theme names, so every mark of its kind is the same
    // cell wherever it stands. A toolbar raises that token to its own row's height (main.css).
    button: {
      variants: {
        square: {
          true: "size-(--bmo-control-square) shrink-0 p-0"
        }
      }
    },
    // A toolbar is one row of equal cells: the square above is as tall as the row, and the row is
    // not clipped, so the shared focus ring stands outside its cells as it does everywhere else.
    dashboardToolbar: {
      slots: {
        root: "overflow-visible [--bmo-control-square:var(--bmo-toolbar-height)]",
        // A target field on the left shrinks with the row rather than pushing the actions off it;
        // the actions themselves keep their cells.
        left: "min-w-0",
        right: "shrink-0"
      }
    },
    // Every badge carries the class that darkens its colours in the light theme (main.css), so its
    // words read at 4.5:1 whatever shape and colour it is given. Nothing is drawn under 14px.
    badge: {
      slots: {
        base: "badge-ink"
      },
      variants: {
        size: {
          xs: { base: "text-sm/4 px-1 py-0 gap-1 rounded-sm" },
          sm: { base: "text-sm/4 px-1.5 py-0.5 gap-1 rounded-sm" }
        }
      }
    },
    // A tooltip's words are the only words that say what an icon button does; they take the caption
    // size in the box the library already gave them. A tooltip that carries a whole cut line wraps
    // inside the room a phone has rather than being cut a second time, and it stands over the
    // message pill, whose own layer has a number of its own.
    tooltip: {
      slots: {
        content: "z-[110] h-auto min-h-6 max-w-[calc(100vw-1rem)] text-sm/4",
        text: "text-clip whitespace-normal break-words text-start"
      }
    },
    select: { slots: dropdown("min-w-[min(var(--reka-select-trigger-width),calc(100vw-1rem))]") },
    selectMenu: { slots: dropdown("min-w-[min(var(--reka-combobox-trigger-width),calc(100vw-1rem))]") },
    // A sheet reaching the foot of the screen keeps what it holds above a phone's gesture bar.
    slideover: {
      compoundVariants: [{ side: ["bottom", "left", "right"], class: { content: "pb-(--bmo-safe-bottom)" } }]
    },
    // A popover never runs past the room the popper measured for it; past that it scrolls.
    popover: {
      slots: {
        content:
          "max-h-(--reka-popover-content-available-height) max-w-(--reka-popover-content-available-width) overflow-y-auto overscroll-contain"
      }
    },
    // A box to write in stands the width of what holds it, wherever it stands.
    textarea: {
      slots: {
        root: "w-full"
      }
    }
  }
});
