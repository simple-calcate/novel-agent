import { SLOT_PROMPTS, WriterMode } from "../editor/guide";

interface Props {
  mode: WriterMode;
  title: string;
  body: string;
  onInsertSlot: (text: string) => void;
}

/** 写在编辑器里的协议：告诉作者此刻该写什么。 */
export function WritingGuide({ mode, title, body, onInsertSlot }: Props) {
  return (
    <div className={`writing-guide mode-${mode}`} data-testid="writing-guide">
      <div className="writing-guide-copy">
        <strong>{title}</strong>
        <p>{body}</p>
      </div>
      {mode === "thinking" && (
        <div className="writing-guide-slots">
          {SLOT_PROMPTS.map((slot) => (
            <button
              key={slot.insert}
              type="button"
              className="slot-chip"
              title={slot.hint}
              onMouseDown={(event) => {
                event.preventDefault();
                onInsertSlot(slot.insert);
              }}
            >
              {slot.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
