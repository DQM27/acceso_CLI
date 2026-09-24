import {
  CellStyleModule,
  ClientSideRowModelApiModule,
  ClientSideRowModelModule,
  ColumnApiModule,
  ColumnAutoSizeModule,
  DateFilterModule,
  LocaleModule,
  ModuleRegistry,
  NumberFilterModule,
  QuickFilterModule,
  RowSelectionModule,
  TextFilterModule,
  TooltipModule,
} from "ag-grid-community";

// Registrar al cargar las tablas, después de entrar al panel.
ModuleRegistry.registerModules([
  CellStyleModule,
  ClientSideRowModelApiModule,
  ClientSideRowModelModule,
  ColumnApiModule,
  ColumnAutoSizeModule,
  DateFilterModule,
  LocaleModule,
  NumberFilterModule,
  QuickFilterModule,
  RowSelectionModule,
  TextFilterModule,
  TooltipModule,
]);

