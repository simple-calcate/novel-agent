import { describe, expect, it } from "vitest";
import type { EditorIdleEvent, ParagraphCreatedEvent } from "./index";

describe("event schema", () => {
  it("editor.idle payload has required fields", () => {
    const event: EditorIdleEvent = {
      eventId: "evt-1",
      eventType: "editor.idle",
      schemaVersion: 1,
      occurredAt: new Date().toISOString(),
      projectId: "p1",
      actor: "user",
      source: "editor",
      platform: "linux",
      transactionId: "tx-1",
      revisionBefore: 0,
      revisionAfter: 1,
      payload: {
        idleMs: 2000,
        charsSinceCommit: 150,
        insertedChars: 150,
        deletedChars: 0,
        composing: false,
      },
    };
    expect(event.payload.idleMs).toBe(2000);
    expect(event.payload.composing).toBe(false);
  });

  it("paragraph.created payload has block info", () => {
    const event: ParagraphCreatedEvent = {
      eventId: "evt-2",
      eventType: "paragraph.created",
      schemaVersion: 1,
      occurredAt: new Date().toISOString(),
      projectId: "p1",
      actor: "user",
      source: "editor",
      platform: "linux",
      transactionId: "tx-2",
      revisionBefore: 1,
      revisionAfter: 2,
      payload: {
        blockId: "b1",
        position: 3,
        insertedChars: 240,
        source: "typing",
      },
    };
    expect(event.payload.blockId).toBe("b1");
    expect(event.payload.source).toBe("typing");
  });
});
