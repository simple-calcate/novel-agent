export interface CaretBox {
  top: number;
  left: number;
  bottom: number;
}

export interface ViewportBox {
  width: number;
  height: number;
}

/** 把菜单贴在光标下方；空间不够则翻到上方，并避免超出视口。 */
export function placeFloatingMenu(
  caret: CaretBox,
  opts: { width?: number; height?: number; viewport?: ViewportBox; gap?: number; pad?: number } = {},
): { top: number; left: number; width: number } {
  const width = opts.width ?? 240;
  const height = opts.height ?? 260;
  const gap = opts.gap ?? 6;
  const pad = opts.pad ?? 8;
  const viewport = opts.viewport ?? {
    width: typeof window === "undefined" ? 1280 : window.innerWidth,
    height: typeof window === "undefined" ? 800 : window.innerHeight,
  };

  const fitsBelow = caret.bottom + gap + height <= viewport.height - pad;
  const fitsAbove = caret.top - gap - height >= pad;
  const top = !fitsBelow && fitsAbove ? caret.top - gap - height : caret.bottom + gap;

  let left = caret.left;
  if (left + width > viewport.width - pad) left = viewport.width - pad - width;
  if (left < pad) left = pad;

  return {
    top: Math.max(pad, Math.min(top, viewport.height - pad - Math.min(height, viewport.height - pad * 2))),
    left,
    width,
  };
}
