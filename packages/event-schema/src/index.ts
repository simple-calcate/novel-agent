export const EVENT_SCHEMA_VERSION = 1 as const;

export type Platform = "linux" | "windows" | "android" | "unknown";

export interface EventEnvelope<TPayload = unknown> {
  eventId: string;
  eventType: string;
  schemaVersion: number;
  occurredAt: string;
  projectId: string;
  bookId?: string;
  chapterId?: string;
  sceneId?: string;
  blockId?: string;
  actor: "user" | "agent" | "plugin" | "system" | "import";
  source: "editor" | "importer" | "agent" | "plugin" | "workflow" | "sync" | "system";
  platform: Platform;
  transactionId: string;
  correlationId?: string;
  causationId?: string;
  revisionBefore: number;
  revisionAfter: number;
  payload: TPayload;
}

export interface EditorIdlePayload {
  idleMs: number;
  charsSinceCommit: number;
  insertedChars: number;
  deletedChars: number;
  cursorBlockId?: string;
  selection?: { from: number; to: number };
  composing: boolean;
}

export interface ParagraphCreatedPayload {
  blockId: string;
  parentBlockId?: string;
  position: number;
  previousBlockId?: string;
  nextBlockId?: string;
  insertedChars: number;
  source: "typing" | "paste" | "import" | "agent";
}

export interface ChapterCreatedPayload {
  chapterId: string;
  bookId: string;
  volumeId?: string;
  position: number;
  title: string;
  templateId?: string;
  previousChapterId?: string;
  source: "user" | "import" | "agent";
}

export interface ContentChangedPayload {
  affectedBlockIds: string[];
  insertedChars: number;
  deletedChars: number;
  operationIds: string[];
}

export type EditorIdleEvent = EventEnvelope<EditorIdlePayload> & {
  eventType: "editor.idle";
};
export type ParagraphCreatedEvent = EventEnvelope<ParagraphCreatedPayload> & {
  eventType: "paragraph.created";
};
export type ChapterCreatedEvent = EventEnvelope<ChapterCreatedPayload> & {
  eventType: "chapter.created";
};
export type ContentChangedEvent = EventEnvelope<ContentChangedPayload> & {
  eventType: "content.changed";
};
