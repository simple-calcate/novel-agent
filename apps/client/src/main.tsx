import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./styles.css";

// 注意：不使用 React.StrictMode。
// Tiptap 的 ReactNodeViewRenderer 在 React 18 开发模式双挂载（mount→unmount→mount）
// 下删除/合并块时会偶发 "NodeView contentDOM 已被移除" 崩溃（webkit webview 尤甚）。
// 项目不依赖 StrictMode 的双调用语义，关闭以获得稳定编辑体验。
ReactDOM.createRoot(document.getElementById("root")!).render(<App />);
