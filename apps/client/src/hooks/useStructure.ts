import { useCallback, useEffect, useState } from "react";
import { libraryApi } from "../api";
import { logger } from "../logger";
import { Project, StoryEntry, StoryEntryKind } from "../types";

export function useStructure(project: Project | null) {
  const [entries, setEntries] = useState<StoryEntry[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(
    async (projectId?: string) => {
      const id = projectId ?? project?.id;
      if (!id) {
        setEntries([]);
        return;
      }
      try {
        setEntries(await libraryApi.listStoryEntries(id));
        setError(null);
      } catch (err) {
        logger.warn("结构列表拉取失败", { error: String(err) });
        setError(String(err));
      }
    },
    [project],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const create = useCallback(
    async (kind: StoryEntryKind, title: string, summary: string) => {
      if (!project) {
        setError("请先选择作品");
        return;
      }
      setBusy(true);
      try {
        await libraryApi.createStoryEntry(project.id, kind, title, summary);
        await refresh();
      } catch (err) {
        logger.error("添加结构失败", { error: String(err) });
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [project, refresh],
  );

  const remove = useCallback(
    async (entry: StoryEntry) => {
      if (!project) return;
      setBusy(true);
      try {
        await libraryApi.deleteStoryEntry(project.id, entry.id, entry.kind);
        await refresh();
      } catch (err) {
        logger.error("删除结构失败", { error: String(err) });
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [project, refresh],
  );

  return { entries, busy, error, create, remove, refresh };
}
