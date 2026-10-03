import {
  CellStyleModule,
  ClientSideRowModelApiModule,
  ClientSideRowModelModule,
  ColumnApiModule,
  ColumnAutoSizeModule,
  ColumnHoverModule,
  DateFilterModule,
  HighlightChangesModule,
  InfiniteRowModelModule,
  LocaleModule,
  ModuleRegistry,
  NumberFilterModule,
  PaginationModule,
  QuickFilterModule,
  RowApiModule,
  RowSelectionModule,
  TextFilterModule,
  TooltipModule,
} from "ag-grid-community";

// Registrar al cargar las tablas, después de entrar al panel. Cada función de
// AG Grid que se usa necesita su módulo acá: si falta, AG Grid no falla fuerte,
// sólo registra el error #200 en la consola y la función no hace nada.
// `InfiniteRowModelModule` + `PaginationModule` son las tablas paginadas en el
// servidor (Historial): la grilla pide cada página con su filtro y su orden.
// `HighlightChangesModule` es el destello de las celdas que cambian
// (`enableCellChangeFlash` en Tabla.tsx, tablas que se refrescan en vivo).
ModuleRegistry.registerModules([
  CellStyleModule,
  ClientSideRowModelApiModule,
  ClientSideRowModelModule,
  ColumnApiModule,
  ColumnAutoSizeModule,
  ColumnHoverModule,
  DateFilterModule,
  HighlightChangesModule,
  InfiniteRowModelModule,
  LocaleModule,
  NumberFilterModule,
  PaginationModule,
  QuickFilterModule,
  RowApiModule,
  RowSelectionModule,
  TextFilterModule,
  TooltipModule,
]);
