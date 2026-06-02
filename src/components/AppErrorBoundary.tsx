import { Component, type ErrorInfo, type ReactNode } from "react";

interface AppErrorBoundaryProps {
  children: ReactNode;
}

interface AppErrorBoundaryState {
  errorMessage: string | null;
}

export class AppErrorBoundary extends Component<AppErrorBoundaryProps, AppErrorBoundaryState> {
  state: AppErrorBoundaryState = { errorMessage: null };

  static getDerivedStateFromError(error: unknown): AppErrorBoundaryState {
    return {
      errorMessage: error instanceof Error ? error.message : String(error),
    };
  }

  componentDidCatch(error: unknown, errorInfo: ErrorInfo) {
    console.error("Waey UI crashed", error, errorInfo);
  }

  render() {
    if (!this.state.errorMessage) {
      return this.props.children;
    }

    return (
      <div className="app-shell theme-dark">
        <div className="panel">
          <div className="panel-header">
            <div className="panel-label">Runtime error</div>
            <div className="panel-title">Waey recovered the overlay</div>
          </div>
          <div className="error-msg">{this.state.errorMessage}</div>
        </div>
      </div>
    );
  }
}
