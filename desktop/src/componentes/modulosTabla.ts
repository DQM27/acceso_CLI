import {
  CellStyleModule,
  ClientSideRowModelApiModule,
  ClientSideRowModelModule,
  ColumnApiModule,
  ColumnAutoSizeModule,
  ColumnHoverModule,
  CsvExportModule,
  DateFilterModule,
  HighlightChangesModule,
  LocaleModule,
  ModuleRegistry,
  NumberFilterModule,
  QuickFilterModule,
  RowApiModule,
  RowSelectionModule,
  RowStyleModule,
  TextFilterModule,
  TooltipModule,
} from "ag-grid-community";

// Registrar al cargar las tablas, después de entrar al panel. Cada función
// de AG Grid que se usa necesita su módulo acá: si falta, AG Grid no falla
// fuerte, sólo registra el error #200 en la consola y la función no hace
// nada (así pasó con RowApi/RowStyle: el resaltado de más de 12 horas de
// Activos nunca se aplicaba).
ModuleRegistry.registerModules([
  CellStyleModule,
  ClientSideRowModelApiModule,
  ClientSideRowModelModule,
  ColumnApiModule,
  ColumnAutoSizeModule,
  ColumnHoverModule,
  CsvExportModule,
  DateFilterModule,
  HighlightChangesModule,
  LocaleModule,
  NumberFilterModule,
  QuickFilterModule,
  RowApiModule,
  RowSelectionModule,
  RowStyleModule,
  TextFilterModule,
  TooltipModule,
]);

