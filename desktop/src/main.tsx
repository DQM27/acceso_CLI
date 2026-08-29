import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { AllCommunityModule, ModuleRegistry } from "ag-grid-community";
import App from "./App";
import ErrorBoundary from "./componentes/ErrorBoundary";
import "./index.css";

ModuleRegistry.registerModules([AllCommunityModule]);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </StrictMode>,
);
