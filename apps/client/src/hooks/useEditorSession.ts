import { useCallback, useEffect, useRef, useState } from "react";
import { libraryApi, isTauriRuntime } from "../api";
import { matchStoryEntries } from "../structure/match";
import {
  Chapter,
  ContentBlock,
  ContextHint,
  ModelConfig,
  PreferenceRule,
  Project,
  StoryEntry,
} from "../types";
import { logger } from "../logger";

interface HintPrefs {
  pinned: string[];
  ignored: string[];
}

function prefsKey(projectId: string): string {
  return `moshu.hintPrefs.${projectId}`;
}

function readPrefs(projectId: string): HintPrefs {
  try {
    const raw = localStorage.getItem(prefsKey(projectId));
    if (!raw) return { pinned: [], ignored: [] };
    const parsed = JSON.parse(raw) as HintPrefs;
    return {
      pinned: parsed.pinned ?? [],
      ignored: parsed.ignored ?? [],
    };
  } catch {
    return { pinned: [], ignored: [] };
  }
}

export function useEditorSession(options: {
  project: Project | null;
  chapters: Chapter[];
  activeChapter: string | null;
  setActiveBookId: (id: string | null) => void;
  storyEntries: StoryEntry[];
}) {
  const { project, chapters, activeChapter, setActiveBookId, storyEntries } = options;
  const [chapterText, setChapterText] = useState("");
  const [chapterBlocks, setChapterBlocks] = useState<ContentBlock[]>([]);
  const [chapterReady, setChapterReady] = useState(false);
  const [hints, setHints] = useState<ContextHint[]>([]);
  const [aiPreview, setAiPreview] = useState("");
  const [revision, setRevision] = useState(0);
  const [preferences, setPreferences] = useState<PreferenceRule[]>([]);
  const [modelConfig, setModelConfig] = useState<ModelConfig | null>(null);
  const [hintPrefs, setHintPrefs] = useState<HintPrefs>({ pinned: [], ignored: [] });
  const draftText = useRef("");
  const draftBlocks = useRef<ContentBlock[]>([]);
  const nearbyRef = useRef({ current: "", previous: "" });

  useEffect(() => {
    libraryApi
      .loadModelConfig()
      .then((config) => {
        if (config) {
          logger.info("恢复模型配置", { provider: config.provider, model: config.model });
          setModelConfig({ ...config, apiKey: "" });
        }
      })
      .catch((e) => logger.error("加载配置失败", { error: String(e) }));
  }, []);

  useEffect(() => {
    if (!project) {
      setPreferences([]);
      setHintPrefs({ pinned: [], ignored: [] });
      return;
    }
    setHintPrefs(readPrefs(project.id));
    libraryApi
      .listPreferences(project.id)
      .then(setPreferences)
      .catch(() => setPreferences([]));
  }, [project]);

  const persistHintPrefs = useCallback(
    (next: HintPrefs) => {
      setHintPrefs(next);
      if (!project) return;
      localStorage.setItem(prefsKey(project.id), JSON.stringify(next));
    },
    [project],
  );

  const pinHint = useCallback(
    (id: string) => {
      persistHintPrefs({
        pinned: hintPrefs.pinned.includes(id)
          ? hintPrefs.pinned.filter((item) => item !== id)
          : [...hintPrefs.pinned, id],
        ignored: hintPrefs.ignored,
      });
    },
    [hintPrefs, persistHintPrefs],
  );

  const ignoreHint = useCallback(
    (id: string) => {
      persistHintPrefs({
        pinned: hintPrefs.pinned.filter((item) => item !== id),
        ignored: hintPrefs.ignored.includes(id) ? hintPrefs.ignored : [...hintPrefs.ignored, id],
      });
    },
    [hintPrefs, persistHintPrefs],
  );

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
    async (nearbyText: string, lookbackText = "") => {
      nearbyRef.current = { current: nearbyText, previous: lookbackText };
      if (!activeChapter) {
        setHints([]);
        return;
      }
      if (!project || !isTauriRuntime()) {
        setHints(matchStoryEntries(nearbyText, lookbackText, storyEntries, revision));
        return;
      }
      logger.debug("刷新上下文提示", { revision, textLength: nearbyText.length });
      try {
        const data = await libraryApi.contextHints({
          projectId: project.id,
          chapterId: activeChapter,
          revision,
          nearbyText,
          lookbackText,
          generation: Date.now(),
        });
        logger.info("上下文提示更新", { count: data.length });
        setHints(data);
      } catch (e) {
        logger.warn("上下文提示走本地匹配", { error: String(e) });
        setHints(matchStoryEntries(nearbyText, lookbackText, storyEntries, revision));
      }
    },
    [project, activeChapter, revision, storyEntries],
  );

  useEffect(() => {
    const nearby = nearbyRef.current.current || nearbyFromText(draftText.current);
    const lookback = nearbyRef.current.previous;
    void refreshHints(nearby, lookback);
  }, [refreshHints, chapterReady]);

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
      const result = await libraryApi.generateContinuation({
        chapterId: activeChapter,
        revision,
        prompt: "继续当前剧情",
        contextText: draftText.current.slice(-800),
        config: modelConfig,
      });
      if (result?.operations?.[0]?.text) {
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
    const text = aiPreview;
    logger.info("拒绝 AI 续写");
    setAiPreview("");
    if (!project || !text) return;
    libraryApi
      .recordGenerationFeedback(project.id, false, text, "", draftText.current.slice(-400))
      .then(setPreferences)
      .catch((error) => logger.warn("记录写作偏好失败", { error: String(error) }));
  }, [aiPreview, project]);

  const togglePreference = useCallback(
    async (rule: PreferenceRule, disabled: boolean) => {
      if (!project) return;
      try {
        setPreferences(await libraryApi.setPreferenceStatus(project.id, rule.id, disabled));
      } catch (error) {
        logger.warn("更新偏好失败", { error: String(error) });
      }
    },
    [project],
  );

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
    preferences,
    togglePreference,
    hintPrefs,
    pinHint,
    ignoreHint,
  };
}

function nearbyFromText(text: string): string {
  const parts = text
    .split(/\n+/)
    .map((item) => item.trim())
    .filter(Boolean);
  return parts[0] ?? "";
}
