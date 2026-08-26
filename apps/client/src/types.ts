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

export interface Book {
  id: string;
  projectId: string;
  title: string;
  synopsis: string;
  position: number;
}

export interface Volume {
  id: string;
  bookId: string;
  title: string;
  position: number;
}

export interface Chapter {
  id: string;
  bookId: string;
  volumeId?: string | null;
  title: string;
  position: number;
  currentRevision: number;
  status: "draft" | "completed" | "archived";
}

export interface Scene {
  id: string;
  chapterId: string;
  title: string;
  position: number;
  povEntryId?: string | null;
}

export interface LibrarySnapshot {
  projects: Project[];
  activeProjectId?: string | null;
  books: Book[];
  volumes?: Volume[];
  chapters: Chapter[];
  scenes?: Scene[];
}

export interface ChapterBody {
  chapterId: string;
  revision: number;
  text: string;
  blocks: ContentBlock[];
}

export type BlockKind = "body" | "thinking";

export type MarkupRef =
  | { type: "task"; id: string; label: string; status: string }
  | { type: "setting"; entityPath: string; field: string; value: string }
  | { type: "custom"; tag: string; body: string };

export interface ContentBlock {
  id: string;
  kind: BlockKind;
  text: string;
  position: number;
  markup: MarkupRef[];
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
  actions?: Array<"expandSource" | "pin" | "snooze" | "ignore" | "markWrong">;
}

export interface JobView {
  id: string;
  operation: string;
  status:
    | "pending"
    | "blocked"
    | "running"
    | "succeeded"
    | "failed"
    | "cancelled"
    | "deadLetter";
  attempts: number;
  createdAt: string;
  updatedAt: string;
}

export type EntityKind =
  | "character"
  | "location"
  | "organization"
  | "item"
  | "ability"
  | "worldRule";

export type FactStatus = "candidate" | "accepted" | "rejected" | "superseded";

export interface CanonProposal {
  factId: string;
  entityId: string;
  projectId: string;
  chapterId?: string | null;
  entityName: string;
  entityKind: EntityKind;
  predicate: string;
  object: string;
  quote: string;
  status: FactStatus;
  confidence: number;
}

export type StoryEntryKind = "character" | "setting" | "foreshadow";

export interface StoryEntry {
  id: string;
  projectId: string;
  kind: StoryEntryKind;
  title: string;
  summary: string;
  aliases: string[];
}

export type PreferenceStatus = "candidate" | "confirmed" | "disabled";

export interface PreferenceRule {
  id: string;
  scope: { projectId?: string } | string;
  rule: string;
  status: PreferenceStatus;
  evidenceProposals?: string[];
  createdAt: string;
  updatedAt: string;
}

export interface PluginSummary {
  id: string;
  name: string;
  version: string;
  runtime: string;
  operations: string[];
}

export interface ModelConfig {
  provider: "openai" | "anthropic" | "deepseek" | "ollama" | "custom";
  apiKey: string;
  baseUrl: string;
  model: string;
  apiKeySet?: boolean;
}

export interface ContinuationPatch {
  operations: Array<{ text?: string }>;
}
