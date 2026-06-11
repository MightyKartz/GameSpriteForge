import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./styles/app.css";

type BootErrorState = {
  error: Error | null;
};

class BootErrorBoundary extends React.Component<React.PropsWithChildren, BootErrorState> {
  state: BootErrorState = { error: null };

  static getDerivedStateFromError(error: Error): BootErrorState {
    return { error };
  }

  componentDidCatch(error: Error) {
    console.error(error);
  }

  render() {
    if (this.state.error) {
      return (
        <main className="boot-error" role="main" aria-label="Startup error">
          <h1>Game Sprite Forge</h1>
          <p>Startup failed.</p>
          <code>{this.state.error.message}</code>
        </main>
      );
    }

    return this.props.children;
  }
}

function renderBootError(error: unknown) {
  const root = document.getElementById("root");
  const message = error instanceof Error ? error.message : String(error);
  if (root) {
    root.innerHTML = [
      '<main class="boot-error" role="main" aria-label="Startup error">',
      "<h1>Game Sprite Forge</h1>",
      "<p>Startup failed.</p>",
      `<code>${escapeHtml(message)}</code>`,
      "</main>",
    ].join("");
  }
}

function escapeHtml(value: string) {
  return value.replace(/[&<>"']/g, (character) => {
    switch (character) {
      case "&":
        return "&amp;";
      case "<":
        return "&lt;";
      case ">":
        return "&gt;";
      case '"':
        return "&quot;";
      default:
        return "&#39;";
    }
  });
}

try {
  const rootElement = document.getElementById("root");
  if (!rootElement) {
    throw new Error("Root element #root was not found.");
  }

  ReactDOM.createRoot(rootElement).render(
    <React.StrictMode>
      <BootErrorBoundary>
        <App />
      </BootErrorBoundary>
    </React.StrictMode>,
  );
} catch (error) {
  console.error(error);
  renderBootError(error);
}
