import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import {
  CellStyleModule,
  ClientSideRowModelApiModule,
  ClientSideRowModelModule,
  ColumnApiModule,
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
// autoajuste de ancho, selección múltiple, `cellStyle` por columna) --
// orden/resize/mover/pin de columnas y sort son parte del núcleo, no
// necesitan módulo aparte. `AllCommunityModule` traía TODO AG Grid Community
// (exportación, paginación, edición avanzada, gráficos, etc.) sin que nada
// de esto se use, inflando el bundle.
//
// `ColumnApiModule` (getColumnState/applyColumnState) y
// `ClientSideRowModelApiModule` (forEachNodeAfterFilter) SÍ hacen falta --
// `Tabla.tsx` los usa para `columnasVisibles()`/`filasFiltradas()` (export a
// Excel/PDF respetando columnas ocultas y filtro actual). Sin registrar,
// esos métodos devuelven vacío en silencio (ningún error en consola), lo que
// hacía fallar el export con "No hay columnas visibles" sin importar qué
// filtro se aplicara.
ModuleRegistry.registerModules([
  CellStyleModule,
  ClientSideRowModelApiModule,
  ClientSideRowModelModule,
  ColumnApiModule,
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
