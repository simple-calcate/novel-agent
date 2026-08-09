export interface CommandResult<T> {
  ok: boolean;
  data?: T;
  error?: string;
}

export interface Project {
  id: string;
  title: string;
  createdAt: string;
  updatedAt: string;
}

export interface Chapter {
  id: string;
  bookId: string;
  title: string;
  position: number;
  currentRevision: number;
  status: "draft" | "completed" | "archived";
}

export interface ContextHint {
  id: string;
  kind:
    | "characterState"
    | "worldRule"
    | "timelineConstraint"
    | "openForeshadowing"
    | "plotHook"
    | "preference"
    | "continuityRisk";
  title: string;
  summary: string;
  sourceLabel: string;
  matchReason: string;
  confidence: number;
  score: number;
  generation: number;
  revision: number;
}

export interface JobView {
  id: string;
  operation: string;
  status: string;
  priority: number;
  attempts: number;
}
