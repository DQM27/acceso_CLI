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

// Registrar al cargar las tablas, después de entrar al panel.
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

