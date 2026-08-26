import { useCallback, useEffect, useState } from "react";
import { libraryApi } from "../api";
import { logger } from "../logger";
import { CanonProposal, Project } from "../types";

export function useCanon(project: Project | null, chapterId: string | null) {
  const [candidates, setCandidates] = useState<CanonProposal[]>([]);
  const [accepted, setAccepted] = useState<CanonProposal[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!project) {
      setCandidates([]);
      setAccepted([]);
      return;
    }
    try {
      const [nextCandidates, nextAccepted] = await Promise.all([
        libraryApi.listCanon(project.id, "candidate"),
        libraryApi.listCanon(project.id, "accepted"),
      ]);
      setCandidates(nextCandidates);
      setAccepted(nextAccepted);
      setError(null);
    } catch (err) {
      logger.warn("正史列表拉取失败", { error: String(err) });
      setError(String(err));
    }
  }, [project]);

  useEffect(() => {
    void refresh();
  }, [refresh, chapterId]);

  const proposeFromChapter = useCallback(async () => {
    if (!chapterId) {
      setError("请先打开一章");
      return;
    }
    setBusy(true);
    try {
      await libraryApi.proposeCanon(chapterId);
      await refresh();
    } catch (err) {
      logger.error("正史抽取失败", { error: String(err) });
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }, [chapterId, refresh]);

  const review = useCallback(
    async (factId: string, accept: boolean) => {
      setBusy(true);
      try {
        await libraryApi.reviewCanonFact(factId, accept);
        await refresh();
      } catch (err) {
        logger.error("正史审核失败", { error: String(err) });
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [refresh],
  );

  return { candidates, accepted, busy, error, proposeFromChapter, review, refresh };
}
