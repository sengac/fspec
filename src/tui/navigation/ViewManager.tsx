/**
 * ViewManager Component
 *
 * Manages navigation between views with history-based routing.
 * Adapted from cage's ViewManager pattern for fspec's TUI infrastructure.
 *
 * Coverage: ITF-001 - Scenario: ViewManager navigates between views with back support
 */

import React, { useState, useContext, createContext } from 'react';

interface ViewMetadata {
  title: string;
  footer?: string;
}

interface ViewDefinition {
  id: string;
  component: React.FC;
  metadata: ViewMetadata;
}

interface ViewManagerContextValue {
  currentView: string;
  navigate: (viewId: string) => void;
  goBack: () => void;
  canGoBack: boolean;
}

const ViewManagerContext = createContext<ViewManagerContextValue | undefined>(
  undefined
);

interface ViewManagerProps {
  initialView: string;
  views: Record<string, ViewDefinition>;
}

export const ViewManager: React.FC<ViewManagerProps> = ({
  initialView,
  views,
}) => {
  const [history, setHistory] = useState<string[]>([initialView]);

  const currentView = history[history.length - 1];

  const navigate = (viewId: string): void => {
    setHistory(prev => [...prev, viewId]);
  };

  const goBack = (): void => {
    if (history.length > 1) {
      setHistory(prev => prev.slice(0, -1));
    }
  };

  const canGoBack = history.length > 1;

  const contextValue: ViewManagerContextValue = {
    currentView,
    navigate,
    goBack,
    canGoBack,
  };

  const view = views[currentView];
  const ViewComponent = view ? view.component : () => null;

  return (
    <ViewManagerContext.Provider value={contextValue}>
      <ViewComponent />
    </ViewManagerContext.Provider>
  );
};

export const useViewManager = (): ViewManagerContextValue => {
  const context = useContext(ViewManagerContext);
  if (!context) {
    throw new Error('useViewManager must be used within ViewManager');
  }
  return context;
};
