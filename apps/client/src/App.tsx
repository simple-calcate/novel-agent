import { useCallback, useEffect, useState } from "react";
import {
  BookOpen,
  Brain,
  CheckCircle2,
  ChevronRight,
  CircleDot,
  FileText,
  Layers,
  ListChecks,
  MessageSquare,
  PenLine,
  Plus,
  Settings,
  Sparkles,
  Terminal,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { CommandResult, ContextHint, Project } from "./types";
import { Editor, AIPreview } from "./components/Editor";
import { ContextRail } from "./components/ContextRail";
import { WorkflowPanel } from "./components/WorkflowPanel";
import { SettingsModal, ModelConfig } from "./components/SettingsModal";
import { LogPanel } from "./components/LogPanel";
import { logger } from "./logger";

const demoProject: Project = {
  id: "00000000-0000-0000-0000-000000000001",
  title: "夜航星图",
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
};

const demoChapters = [
  { id: "c1", title: "第一章 雾港来客", position: 1, status: "draft" },
  { id: "c2", title: "第二章 旧王玺", position: 2, status: "draft" },
  { id: "c3", title: "第三章 潮下之城", position: 3, status: "draft" },
];

export function App() {
  const [project] = useState<Project>(demoProject);
  const [activeChapter, setActiveChapter] = useState("c1");
  const [hints, setHints] = useState<ContextHint[]>([]);
  const [aiPreview, setAiPreview] = useState("");
  const [sidebarTab, setSidebarTab] = useState<"context" | "workflow" | "agent">("context");
  const [jobs, setJobs] = useState<Array<{ id: string; label: string; status: string }>>([]);
  const [revision, setRevision] = useState(0);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [logPanelOpen, setLogPanelOpen] = useState(false);
  const [_modelConfig, setModelConfig] = useState<ModelConfig | null>(null);

  // 启动时加载配置
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

  const refreshHints = useCallback(
    async (nearbyText: string) => {
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
    [project.id, activeChapter, revision],
  );

  useEffect(() => {
    setHints(buildLocalHints("", revision));
  }, [revision]);

  const addJob = useCallback((label: string) => {
    logger.info("添加任务", { label });
    setJobs((current) => [
      { id: `${Date.now()}`, label, status: "pending" },
      ...current.slice(0, 7),
    ]);
  }, []);

  const handleGenerate = useCallback(async () => {
    if (!_modelConfig) {
      logger.warn("未配置模型");
      setAiPreview("请先点击左下角「设置」配置 AI 模型");
      return;
    }
    logger.info("开始 AI 续写", { provider: _modelConfig.provider, model: _modelConfig.model });
    addJob("AI 续写");
    try {
      const result = await invoke<{ operations: Array<{ text: string }> }>(
        "generate_continuation",
        {
          chapterId: activeChapter,
          revision,
          prompt: "继续当前剧情",
          contextText: "雾港、旧王玺、失忆的航海师",
          config: _modelConfig,
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
  }, [_modelConfig, activeChapter, revision, addJob]);

  const handleAccept = useCallback(() => {
    if (aiPreview && (window as any).__editorInsert) {
      (window as any).__editorInsert(aiPreview);
      logger.info("接受 AI 续写");
      setAiPreview("");
    }
  }, [aiPreview]);

  const handleReject = useCallback(() => {
    logger.info("拒绝 AI 续写");
    setAiPreview("");
  }, []);

  return (
    <div className="app-shell">
      {/* 左侧边栏 */}
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <PenLine size={20} />
          </div>
          <div>
            <div className="brand-name">墨枢</div>
            <div className="brand-sub">Novel Agent</div>
          </div>
        </div>

        <div className="project-card">
          <div className="project-label">当前作品</div>
          <div className="project-title">{project.title}</div>
          <div className="project-meta">
            <span>本地优先</span>
            <span>·</span>
            <span>自动保存</span>
          </div>
        </div>

        <nav className="tree">
          <div className="tree-section">
            <Layers size={14} />
            <span>卷一 · 雾与海</span>
          </div>
          {demoChapters.map((chapter) => (
            <button
              key={chapter.id}
              className={`tree-item ${activeChapter === chapter.id ? "active" : ""}`}
              onClick={() => setActiveChapter(chapter.id)}
            >
              <FileText size={14} />
              <span>{chapter.title}</span>
              <ChevronRight size={12} className="tree-arrow" />
            </button>
          ))}
          <button className="tree-item add">
            <Plus size={14} />
            <span>新章节</span>
          </button>
        </nav>

        <div className="sidebar-footer">
          <button className="icon-button" title="设置" onClick={() => setSettingsOpen(true)}>
            <Settings size={16} />
          </button>
          <button className="icon-button" title="插件">
            <Layers size={16} />
          </button>
          <button className="icon-button" title="任务队列">
            <ListChecks size={16} />
          </button>
          <button
            className={`icon-button ${logPanelOpen ? "active" : ""}`}
            title="日志"
            onClick={() => setLogPanelOpen(!logPanelOpen)}
          >
            <Terminal size={16} />
          </button>
        </div>
      </aside>

      {/* 中间编辑区 */}
      <main className="workspace">
        <header className="topbar">
          <div className="chapter-title">
            <BookOpen size={16} />
            <span>{demoChapters.find((c) => c.id === activeChapter)?.title}</span>
            <span className="revision-badge">R{revision}</span>
          </div>
          <div className="topbar-actions">
            <button className="action-button ghost" onClick={() => addJob("章节一致性检查")}>
              <CheckCircle2 size={14} />
              检查
            </button>
            <button className="action-button primary" onClick={handleGenerate}>
              <Sparkles size={14} />
              续写
            </button>
          </div>
        </header>

        <div className="editor-area">
          <Editor
            onTextChange={(text) => {
              setRevision((v) => v + 1);
              refreshHints(text.slice(-300));
            }}
            onIdle={() => addJob("停笔触发：索引与提示刷新")}
          />
          <ContextRail hints={hints} />

          {/* AI 预览卡片 - 浮动在编辑器上方 */}
          {aiPreview && (
            <AIPreview
              text={aiPreview}
              onAccept={handleAccept}
              onReject={handleReject}
              onRevise={handleGenerate}
            />
          )}
        </div>
      </main>

      {/* 右侧面板 */}
      <aside className="right-panel">
        <div className="panel-tabs">
          <button
            className={sidebarTab === "context" ? "active" : ""}
            onClick={() => setSidebarTab("context")}
          >
            <Brain size={14} />
            上下文
          </button>
          <button
            className={sidebarTab === "workflow" ? "active" : ""}
            onClick={() => setSidebarTab("workflow")}
          >
            <ListChecks size={14} />
            工作流
          </button>
          <button
            className={sidebarTab === "agent" ? "active" : ""}
            onClick={() => setSidebarTab("agent")}
          >
            <MessageSquare size={14} />
            Agent
          </button>
        </div>

        {sidebarTab === "context" && (
          <div className="panel-content">
            <h3>当前场景包</h3>
            <div className="context-card">
              <div className="context-card-title">
                <CircleDot size={12} />
                POV 边界
              </div>
              <p>当前视角：沈雾。她不知道旧王玺已在船长手中。</p>
            </div>
            <div className="context-card">
              <div className="context-card-title">
                <CircleDot size={12} />
                未兑现伏笔
              </div>
              <p>雾中灯塔的信号 · 第二章前需要呼应</p>
            </div>
            <div className="context-card">
              <div className="context-card-title">
                <CircleDot size={12} />
                世界规则
              </div>
              <p>潮下之城禁止明火，违者会失去名字。</p>
            </div>
          </div>
        )}

        {sidebarTab === "workflow" && (
          <WorkflowPanel jobs={jobs} onRun={(label) => addJob(label)} />
        )}

        {sidebarTab === "agent" && (
          <div className="panel-content">
            <h3>Agent 会话</h3>
            <div className="agent-message">
              <strong>系统</strong>
              <p>上下文已固定到 Revision {revision}，包含 12 条正史与 3 条伏笔。</p>
            </div>
            <div className="agent-message user">
              <strong>你</strong>
              <p>检查这一章的视角泄漏。</p>
            </div>
            <div className="agent-message">
              <strong>Agent</strong>
              <p>未发现 POV 违规。沈雾在第 4 段只感知到"玺印反光"，未读取其归属。</p>
            </div>
          </div>
        )}
      </aside>

      {/* 模态框 */}
      <SettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        initialConfig={_modelConfig}
        onSave={async (config) => {
          logger.info("保存模型配置", { provider: config.provider, model: config.model });
          setModelConfig(config);
          try {
            await invoke("save_model_config", { config });
            logger.info("配置已同步到后端");
          } catch (e) {
            logger.error("保存失败", { error: String(e) });
          }
        }}
      />

      <LogPanel open={logPanelOpen} onClose={() => setLogPanelOpen(false)} />
    </div>
  );
}

function buildLocalHints(nearby: string, revision: number): ContextHint[] {
  const base: ContextHint[] = [
    {
      id: "h1",
      kind: "characterState",
      title: "沈雾",
      summary: "左手有盐渍伤，见到旧王玺会触发记忆闪回",
      sourceLabel: "人物卡",
      matchReason: "当前章节主角",
      confidence: 0.95,
      score: 0.95,
      generation: revision,
      revision,
    },
    {
      id: "h2",
      kind: "openForeshadowing",
      title: "雾中灯塔",
      summary: "灯塔在每晚第三次潮响时亮起，第二章前需要呼应",
      sourceLabel: "伏笔看板",
      matchReason: "本章尚未提及",
      confidence: 0.8,
      score: 0.82,
      generation: revision,
      revision,
    },
    {
      id: "h3",
      kind: "worldRule",
      title: "潮下城禁令",
      summary: "禁止明火，违者会失去名字",
      sourceLabel: "世界规则",
      matchReason: "场景包含潮下城",
      confidence: 1,
      score: 0.9,
      generation: revision,
      revision,
    },
  ];

  if (nearby.includes("玺")) {
    base.unshift({
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
