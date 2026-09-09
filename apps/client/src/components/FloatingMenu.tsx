import type { CSSProperties, MouseEventHandler, ReactNode } from "react";
import { createPortal } from "react-dom";
import { CaretBox, placeFloatingMenu } from "../editor/menuPlace";

export function FloatingMenu({
  caret,
  itemCount,
  width = 240,
  children,
  onMouseDown,
}: {
  caret: CaretBox;
  itemCount: number;
  width?: number;
  children: ReactNode;
  onMouseDown?: MouseEventHandler<HTMLDivElement>;
}) {
  if (itemCount <= 0 || typeof document === "undefined") return null;
  const height = Math.min(320, 12 + itemCount * 48);
  const placed = placeFloatingMenu(caret, { width, height });
  const style: CSSProperties = {
    position: "fixed",
    top: placed.top,
    left: placed.left,
    width: placed.width,
    zIndex: 1200,
  };
  return createPortal(
    <div className="mention-menu" style={style} onMouseDown={onMouseDown} role="listbox">
      {children}
    </div>,
    document.body,
  );
}
