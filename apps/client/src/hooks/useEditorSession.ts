import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { libraryApi } from "../api";
import { Chapter, CommandResult, ContentBlock, ContextHint, Project } from "../types";
import { logger } from "../logger";
import { ModelConfig } from "../components/SettingsModal";

export function useEditorSession(options: {
  project: Project | null;
  chapters: Chapter[];
  activeChapter: string | null;
  setActiveBookId: (id: string | null) => void;
}) {
  const { project, chapters, activeChapter, setActiveBookId } = options;
  const [chapterText, setChapterText] = useState("");
  const [chapterBlocks, setChapterBlocks] = useState<ContentBlock[]>([]);
  const [chapterReady, setChapterReady] = useState(false);
  const [hints, setHints] = useState<ContextHint[]>([]);
  const [aiPreview, setAiPreview] = useState("");
  const [revision, setRevision] = useState(0);
  const [modelConfig, setModelConfig] = useState<ModelConfig | null>(null);
  const draftText = useRef("");
  const draftBlocks = useRef<ContentBlock[]>([]);

  useEffect(() => {
    invoke<ModelConfig | null>("load_model_config")
      .then((config) => {
        if (config) {
          logger.info("恢复模型配置", { provider: config.provider, model: config.model });
          setModelConfig(config);
        }
      })
      .catch((e) => logger.error("加载配置失败", { error: String(e) }));
  }, []);

  useEffect(() => {
    if (!activeChapter) {
      setChapterText("");
      setChapterReady(false);
      setRevision(0);
      draftText.current = "";
      return;
    }
    let cancelled = false;
    setChapterReady(false);
    libraryApi
      .loadChapter(activeChapter)
      .then((body) => {
        if (cancelled) return;
        setChapterText(body.text);
        setChapterBlocks(body.blocks ?? []);
        draftText.current = body.text;
        draftBlocks.current = body.blocks ?? [];
        setRevision(body.revision);
        setChapterReady(true);
        const chapter = chapters.find((item) => item.id === activeChapter);
        if (chapter) setActiveBookId(chapter.bookId);
      })
      .catch((error) => {
        if (cancelled) return;
        logger.error("加载章节失败", { error: String(error) });
        setChapterText("");
        setChapterReady(true);
      });
    return () => {
      cancelled = true;
    };
    // 只在切换章节时重新加载，避免作品库刷新打断编辑器。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeChapter]);

  const persistChapter = useCallback(async () => {
    if (!activeChapter) return;
    try {
      const saved = await libraryApi.saveChapter(
        activeChapter,
        draftText.current,
        draftBlocks.current,
      );
      setRevision(saved.revision);
    } catch (error) {
      logger.warn("保存章节失败", { error: String(error) });
    }
  }, [activeChapter]);

  const refreshHints = useCallback(
    async (nearbyText: string) => {
      if (!project || !activeChapter) return;
      logger.debug("刷新上下文提示", { revision, textLength: nearbyText.length });
      try {
        const result = await invoke<CommandResult<ContextHint[]>>("context_hints", {
          input: {
            projectId: project.id,
            chapterId: activeChapter,
            revision,
            nearbyText,
            generation: Date.now(),
          },
        });
        if (result.ok && result.data) {
          logger.info("上下文提示更新", { count: result.data.length });
          setHints(result.data);
        }
      } catch (e) {
        logger.error("上下文提示失败", { error: String(e) });
        setHints(buildLocalHints(nearbyText, revision));
      }
    },
    [project, activeChapter, revision],
  );

  useEffect(() => {
    setHints(buildLocalHints("", revision));
  }, [revision]);

  const handleGenerate = useCallback(async () => {
    if (!modelConfig) {
      logger.warn("未配置模型");
      setAiPreview("请先点击左下角「设置」配置 AI 模型");
      return;
    }
    if (!activeChapter) {
      setAiPreview("请先创建并打开一个章节");
      return;
    }
    logger.info("开始 AI 续写", { provider: modelConfig.provider, model: modelConfig.model });
    try {
      const result = await invoke<{ operations: Array<{ text: string }> }>(
        "generate_continuation",
        {
          chapterId: activeChapter,
          revision,
          prompt: "继续当前剧情",
          contextText: draftText.current.slice(-800),
          config: modelConfig,
        },
      );
      if (result?.operations?.[0]) {
        logger.info("AI 续写成功", { length: result.operations[0].text.length });
        setAiPreview(result.operations[0].text);
      }
    } catch (e) {
      logger.error("AI 续写失败", { error: String(e) });
      setAiPreview(`调用失败: ${e}`);
    }
  }, [modelConfig, activeChapter, revision]);

  const handleAccept = useCallback(() => {
    if (aiPreview && (window as Window & { __editorInsert?: (text: string) => void }).__editorInsert) {
      (window as Window & { __editorInsert?: (text: string) => void }).__editorInsert?.(aiPreview);
      logger.info("接受 AI 续写");
      setAiPreview("");
    }
  }, [aiPreview]);

  const handleReject = useCallback(() => {
    logger.info("拒绝 AI 续写");
    setAiPreview("");
  }, []);

  return {
    chapterText,
    chapterBlocks,
    chapterReady,
    hints,
    aiPreview,
    revision,
    modelConfig,
    setModelConfig,
    draftText,
    draftBlocks,
    persistChapter,
    refreshHints,
    handleGenerate,
    handleAccept,
    handleReject,
  };
}

function buildLocalHints(nearby: string, revision: number): ContextHint[] {
  const base: ContextHint[] = [];
  if (nearby.includes("玺")) {
    base.push({
      id: "h0",
      kind: "plotHook",
      title: "旧王玺",
      summary: "沈雾不知道它已在船长手中；避免提前揭示",
      sourceLabel: "正史",
      matchReason: "当前文字包含「玺」",
      confidence: 0.98,
      score: 1,
      generation: revision,
      revision,
    });
  }
  return base.slice(0, 5);
}
