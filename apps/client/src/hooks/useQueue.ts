import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { CommandResult, JobView, Project } from "../types";
import { logger } from "../logger";
import { operationLabel } from "../workflow/labels";

export function useQueue(project: Project | null) {
  const [jobs, setJobs] = useState<Array<{ id: string; label: string; status: string }>>([]);
  const [queueReady, setQueueReady] = useState(false);
  const jobsRef = useRef(jobs);
  jobsRef.current = jobs;
  const drainingRef = useRef(false);

  const refreshJobs = useCallback(async () => {
    try {
      const result = await invoke<CommandResult<JobView[]>>("list_jobs");
      if (result?.ok && result.data) {
        setJobs(
          result.data.map((job) => ({
            id: job.id,
            label: operationLabel(job.operation),
            status: job.status,
          })),
        );
        setQueueReady(true);
      }
    } catch (e) {
      logger.warn("任务列表拉取失败（可能在纯浏览器预览中）", { error: String(e) });
    }
  }, []);

  const drainQueue = useCallback(async () => {
    if (drainingRef.current) return;
    drainingRef.current = true;
    try {
      for (let i = 0; i < 50; i++) {
        const step = await invoke<CommandResult<{ executed: boolean }>>("run_queue_step");
        if (!step?.ok || !step.data?.executed) break;
        await refreshJobs();
      }
    } catch (e) {
      logger.warn("队列驱动失败（可能在纯浏览器预览中）", { error: String(e) });
    } finally {
      drainingRef.current = false;
    }
  }, [refreshJobs]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen("queue:changed", () => {
      void drainQueue();
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((e) => logger.warn("队列事件监听不可用（纯浏览器预览）", { error: String(e) }));

    void drainQueue();

    const timer = setInterval(() => {
      const hasWork = jobsRef.current.some(
        (j) => j.status === "pending" || j.status === "running",
      );
      if (hasWork) void drainQueue();
    }, 30_000);
    return () => {
      unlisten?.();
      clearInterval(timer);
    };
  }, [drainQueue]);

  const enqueue = useCallback(
    async (operation: string, extraPayload?: Record<string, unknown>) => {
      if (!project) {
        logger.warn("未选择作品，跳过入队", { operation });
        return;
      }
      logger.info("入队任务", { operation });
      const payload = { projectId: project.id, ...extraPayload };
      try {
        const result = await invoke<CommandResult<{ jobId: string }>>("enqueue_job", {
          input: { projectId: project.id, operation, payload, priority: 0 },
        });
        if (!result.ok) {
          logger.error("入队失败", { operation, error: result.error });
        } else {
          void drainQueue();
        }
      } catch (e) {
        logger.error("入队异常", { operation, error: String(e) });
      }
    },
    [project, drainQueue],
  );

  return { jobs, queueReady, enqueue, drainQueue, refreshJobs };
}
