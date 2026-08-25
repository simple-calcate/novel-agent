import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  /** 崩溃区域的标签，用于错误卡片提示 */
  label?: string;
  /** 自定义恢复动作 */
  onReset?: () => void;
}

interface State {
  error: Error | null;
}

/**
 * 渲染树兜底：任何子组件抛错时局部显示错误卡片，
 * 而不是整棵树卸载（表现为全屏灰/白）。
 * Tiptap NodeView 在删除/合并块时偶发崩溃，由这里截获，
 * 用户可点击「重置编辑器」恢复输入，不丢已持久化内容。
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: { componentStack?: string | null }) {
    console.error("[ErrorBoundary]", this.props.label ?? "区域", error, info.componentStack);
  }

  private reset = () => {
    this.setState({ error: null });
    this.props.onReset?.();
  };

  render() {
    if (!this.state.error) return this.props.children;

    return (
      <div className="error-boundary">
        <div className="error-boundary-card">
          <div className="error-boundary-title">
            <span className="error-boundary-icon">⚠</span>
            {this.props.label ?? "界面"}运行出错
          </div>
          <div className="error-boundary-msg">{String(this.state.error.message || this.state.error)}</div>
          <button className="error-boundary-reset" onClick={this.reset}>
            重置此区域
          </button>
        </div>
      </div>
    );
  }
}
