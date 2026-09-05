import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import {
  ClientSideRowModelModule,
  ColumnAutoSizeModule,
  DateFilterModule,
  ModuleRegistry,
  NumberFilterModule,
  QuickFilterModule,
  RowSelectionModule,
  TextFilterModule,
} from "ag-grid-community";
import App from "./App";
import ErrorBoundary from "./componentes/ErrorBoundary";
import "./index.css";

// Solo los módulos que `Tabla.tsx` y las pantallas realmente usan (modelo de
// filas del lado del cliente, filtros de texto/número/fecha, quick filter,
// autoajuste de ancho, selección múltiple) -- orden/resize/mover/pin de
// columnas y sort son parte del núcleo, no necesitan módulo aparte.
// `AllCommunityModule` traía TODO AG Grid Community (exportación,
// paginación, edición avanzada, gráficos, etc.) sin que nada de esto se use,
// inflando el bundle.
ModuleRegistry.registerModules([
  ClientSideRowModelModule,
  ColumnAutoSizeModule,
  DateFilterModule,
  NumberFilterModule,
  QuickFilterModule,
  RowSelectionModule,
  TextFilterModule,
]);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </StrictMode>,
);
