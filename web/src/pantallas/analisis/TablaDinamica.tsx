import { useEffect, useMemo, useRef, useState } from "react";
import {
  ConditionalFormatting,
  DrillThrough,
  ExcelExport,
  FieldList,
  GroupingBar,
  Inject,
  NumberFormatting,
  PDFExport,
  PivotChart,
  PivotViewComponent,
  Toolbar,
  VirtualScroll,
} from "@syncfusion/ej2-react-pivotview";
import type { AggregateEventArgs, ToolbarItems } from "@syncfusion/ej2-react-pivotview";
import type { PivotViewModel } from "@syncfusion/ej2-pivotview";

// El paquete no exporta este tipo por su nombre; se toma de la API pública.
type DataSourceSettingsModel = NonNullable<PivotViewModel["dataSourceSettings"]>;
import type { ResumenDiario } from "../../api/analisis";
import { TEXTO_TIPO_PERSONA } from "../../api/analisis";
import { DIAS_SEMANA, filaDinamica, permanenciaDeCelda } from "./Analisis.logica";
import type { FilaDinamica } from "./Analisis.logica";
import { CULTURA } from "./syncfusion";

/**
 * Tabla dinámica (Syncfusion PivotView) sobre el resumen diario. La persona
 * arrastra campos entre filas, columnas y filtros; el diseño inicial es
 * unidad × mes con ingresos y permanencia promedio.
 *
 * Sin campos calculados a propósito: Syncfusion evalúa sus fórmulas con
 * `Function(...)` (eval), que la CSP del panel bloquea (`script-src 'self'`,
 * sin 'unsafe-eval'). Por eso no se inyecta `CalculatedField` (la persona no
 * puede crearlos desde la lista de campos), y la permanencia promedio se
 * calcula en `aggregateCellInfo` con las filas de cada celda.
 */

const CAMPO_PERMANENCIA = "permanencia";

const DISENO_INICIAL: DataSourceSettingsModel = {
  expandAll: false,
  enableSorting: true,
  showSubTotals: true,
  showGrandTotals: true,
  // Sin "Suma de ..." delante de cada valor: la permanencia es un promedio
  // (ver `alAgregarCelda`) y "Suma de Permanencia promedio" diría lo contrario.
  showAggregationOnValueField: false,
  rows: [{ name: "unidad", caption: "Unidad" }],
  columns: [
    { name: "dia_date_group_years", caption: "Año" },
    { name: "dia_date_group_months", caption: "Mes" },
  ],
  values: [
    { name: "ingresos", caption: "Ingresos", type: "Sum" },
    { name: CAMPO_PERMANENCIA, caption: "Permanencia promedio (min)", type: "Sum" },
  ],
  filters: [{ name: "tipo_persona", caption: "Tipo de persona" }],
  formatSettings: [
    { name: "ingresos", format: "N0" },
    { name: "con_salida", format: "N0" },
    { name: "horas_adentro", format: "N0" },
    { name: CAMPO_PERMANENCIA, format: "N0" },
    { name: "dia", type: "date", format: "dd/MM/yyyy" },
  ],
  groupSettings: [{ name: "dia", type: "Date", groupInterval: ["Years", "Months", "Days"] }],
  sortSettings: [{ name: "dia_semana", membersOrder: [...DIAS_SEMANA] }],
  fieldMapping: [
    { name: "dia", caption: "Día" },
    { name: "dia_semana", caption: "Día de la semana" },
    { name: "unidad", caption: "Unidad" },
    { name: "tipo_persona", caption: "Tipo de persona" },
    { name: "tipo_ingreso", caption: "Tipo de ingreso" },
    { name: "medio", caption: "Medio" },
    { name: "ingresos", caption: "Ingresos" },
    { name: "con_salida", caption: "Salidas registradas" },
    { name: "horas_adentro", caption: "Horas adentro (total)" },
    { name: "minutos_adentro", caption: "Minutos adentro (total)" },
    { name: CAMPO_PERMANENCIA, caption: "Permanencia promedio (min)" },
  ],
};

const BARRA: ToolbarItems[] = [
  "Grid",
  "Chart",
  "Export",
  "SubTotal",
  "GrandTotal",
  "ConditionalFormatting",
  "NumberFormatting",
  "FieldList",
];

export default function TablaDinamica({ diario }: { diario: ResumenDiario[] }) {
  const pivotRef = useRef<PivotViewComponent>(null);
  const filas = useMemo<FilaDinamica[]>(
    () => diario.map((fila) => filaDinamica(fila, TEXTO_TIPO_PERSONA[fila.tipo_persona])),
    [diario],
  );

  // El diseño se fija una sola vez; al cambiar el período o las unidades sólo
  // se reemplazan los datos, así no se pierde lo que la persona acomodó.
  const [disenoInicial] = useState<DataSourceSettingsModel>(() => ({ ...DISENO_INICIAL, dataSource: filas }));
  const primeraCarga = useRef(true);
  useEffect(() => {
    if (primeraCarga.current) {
      primeraCarga.current = false;
      return;
    }
    const pivot = pivotRef.current;
    if (pivot) pivot.dataSourceSettings.dataSource = filas;
  }, [filas]);

  function alAgregarCelda(args: AggregateEventArgs) {
    if (args.fieldName !== CAMPO_PERMANENCIA) return;
    args.value = permanenciaDeCelda(args.cellSets as FilaDinamica[]);
  }

  return (
    <section className="tarjeta analisis-pivot flex min-w-0 flex-col gap-1 p-3">
      <h2 className="text-sm font-semibold text-(--texto)">Tabla dinámica</h2>
      <p className="text-xs text-(--muted)">
        Arrastre campos entre filas, columnas y filtros desde la lista de campos. Permanencia: promedio en minutos
        de quienes ya salieron.
      </p>
      <PivotViewComponent
        id="tabla-dinamica"
        ref={pivotRef}
        locale={CULTURA}
        height={760}
        width="100%"
        dataSourceSettings={disenoInicial}
        aggregateCellInfo={alAgregarCelda}
        cssClass="pivot-lista-campos"
        showToolbar
        toolbar={BARRA}
        showFieldList
        showGroupingBar
        allowExcelExport
        allowPdfExport
        allowConditionalFormatting
        allowNumberFormatting
        allowDrillThrough
        displayOption={{ view: "Both", primary: "Table" }}
        chartSettings={{ chartSeries: { type: "Column" }, enableExport: true }}
        allowCalculatedField={false}
        allowDeferLayoutUpdate
        enableVirtualization
        enableHtmlSanitizer
        gridSettings={{ columnWidth: 120, allowSelection: false }}
      >
        <Inject
          services={[
            FieldList,
            GroupingBar,
            Toolbar,
            ExcelExport,
            PDFExport,
            PivotChart,
            ConditionalFormatting,
            NumberFormatting,
            DrillThrough,
            VirtualScroll,
          ]}
        />
      </PivotViewComponent>
    </section>
  );
}
