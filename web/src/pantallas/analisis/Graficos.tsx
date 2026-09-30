import {
  BarSeries,
  Category,
  ChartComponent,
  ColumnSeries,
  Crosshair,
  DateTime,
  Inject,
  Legend,
  LineSeries,
  SeriesCollectionDirective,
  SeriesDirective,
  Tooltip,
} from "@syncfusion/ej2-react-charts";
import type {
  AxisModel,
  ChartAreaModel,
  LegendSettingsModel,
  TooltipSettingsModel,
} from "@syncfusion/ej2-react-charts";
import type { PuntoHora, PuntoTendencia, ResumenEmpresa } from "../../api/analisis";
import { escaparHtml } from "./Analisis.logica";
import { CULTURA, colorToken, useTemaPanel } from "./syncfusion";

/**
 * Gráficos de la pantalla Análisis. Colores por el papel que cumplen: dos
 * series categóricas (azul y naranja, validadas contra la superficie del
 * panel en claro y en oscuro con el validador de paletas: separación para
 * daltonismo ΔE ≥ 24, contraste ≥ 3:1) y un solo azul cuando hay una sola
 * serie. El texto nunca va del color de la serie: usa los tokens de texto del
 * panel, y la cuadrícula el de borde, para que quede en segundo plano.
 */
const PALETA = {
  light: ["#2a78d6", "#eb6834"],
  dark: ["#3987e5", "#d95926"],
} as const;

interface Estilo {
  serie: readonly [string, string];
  texto: string;
  secundario: string;
  cuadricula: string;
  fuente: string;
}

function useEstilo(): Estilo {
  const tema = useTemaPanel();
  return {
    serie: PALETA[tema],
    texto: colorToken("--texto"),
    secundario: colorToken("--muted"),
    cuadricula: colorToken("--borde"),
    fuente: colorToken("--fuente"),
  };
}

function ejeX(estilo: Estilo, extra: AxisModel): AxisModel {
  return {
    majorGridLines: { width: 0 },
    majorTickLines: { width: 0 },
    lineStyle: { width: 1, color: estilo.cuadricula },
    labelStyle: { color: estilo.secundario, fontFamily: estilo.fuente, size: "12px" },
    ...extra,
  };
}

function ejeY(estilo: Estilo, extra: AxisModel = {}): AxisModel {
  return {
    minimum: 0,
    majorGridLines: { width: 1, color: estilo.cuadricula, dashArray: "2,4" },
    majorTickLines: { width: 0 },
    lineStyle: { width: 0 },
    labelStyle: { color: estilo.secundario, fontFamily: estilo.fuente, size: "12px" },
    labelFormat: "n0",
    ...extra,
  };
}

function leyenda(estilo: Estilo): LegendSettingsModel {
  return {
    visible: true,
    position: "Top",
    alignment: "Near",
    shapeHeight: 10,
    shapeWidth: 10,
    textStyle: { color: estilo.texto, fontFamily: estilo.fuente, size: "12px" },
  };
}

function tooltip(estilo: Estilo, extra: TooltipSettingsModel = {}): TooltipSettingsModel {
  return {
    enable: true,
    fill: colorToken("--elevado"),
    border: { color: estilo.cuadricula, width: 1 },
    textStyle: { color: estilo.texto, fontFamily: estilo.fuente, size: "12px" },
    opacity: 1,
    ...extra,
  };
}

const AREA: ChartAreaModel = { border: { width: 0 } };
const ALTO = "260px";

function Tarjeta({ titulo, detalle, children }: { titulo: string; detalle: string; children: React.ReactNode }) {
  return (
    <section className="tarjeta flex min-w-0 flex-col gap-1 p-3">
      <h2 className="text-sm font-semibold text-(--texto)">{titulo}</h2>
      <p className="text-xs text-(--muted)">{detalle}</p>
      {children}
    </section>
  );
}

/** Ingresos por día: contratistas y proveedores como líneas, no apiladas (sus
 * magnitudes son muy distintas y apiladas se leería mal la de arriba). */
export function GraficoTendencia({ puntos }: { puntos: PuntoTendencia[] }) {
  const estilo = useEstilo();
  const datos = puntos.map((p) => ({ ...p, fecha: new Date(`${p.dia}T00:00:00`) }));
  return (
    <Tarjeta titulo="Ingresos por día" detalle="Contratistas y proveedores, día de Costa Rica.">
      <ChartComponent
        id="grafico-tendencia"
        locale={CULTURA}
        height={ALTO}
        background="transparent"
        enableHtmlSanitizer
        chartArea={AREA}
        primaryXAxis={ejeX(estilo, { valueType: "DateTime", labelFormat: "d MMM", intervalType: "Auto", edgeLabelPlacement: "Shift" })}
        primaryYAxis={ejeY(estilo)}
        legendSettings={leyenda(estilo)}
        tooltip={tooltip(estilo, { shared: true, format: "${series.name}: <b>${point.y}</b>", header: "${point.x}" })}
        crosshair={{ enable: true, lineType: "Vertical", line: { color: estilo.secundario, width: 1 } }}
      >
        <Inject services={[LineSeries, DateTime, Legend, Tooltip, Crosshair]} />
        <SeriesCollectionDirective>
          <SeriesDirective
            dataSource={datos}
            xName="fecha"
            yName="contratistas"
            name="Contratistas"
            type="Line"
            width={2}
            fill={estilo.serie[0]}
          />
          <SeriesDirective
            dataSource={datos}
            xName="fecha"
            yName="proveedores"
            name="Proveedores"
            type="Line"
            width={2}
            fill={estilo.serie[1]}
          />
        </SeriesCollectionDirective>
      </ChartComponent>
    </Tarjeta>
  );
}

/** Horas pico: promedio de ingresos por hora en un día de semana y en uno de
 * fin de semana, lado a lado. */
export function GraficoHoras({ horas }: { horas: PuntoHora[] }) {
  const estilo = useEstilo();
  const esquinas = { topLeft: 4, topRight: 4, bottomLeft: 0, bottomRight: 0 };
  return (
    <Tarjeta titulo="Horas pico" detalle="Ingresos promedio por hora en un día del período.">
      <ChartComponent
        id="grafico-horas"
        locale={CULTURA}
        height={ALTO}
        background="transparent"
        enableHtmlSanitizer
        chartArea={AREA}
        primaryXAxis={ejeX(estilo, { valueType: "Category", interval: 3 })}
        primaryYAxis={ejeY(estilo, { labelFormat: "n1" })}
        legendSettings={leyenda(estilo)}
        tooltip={tooltip(estilo, { shared: true, format: "${series.name}: <b>${point.y}</b>", header: "${point.x}" })}
      >
        <Inject services={[ColumnSeries, Category, Legend, Tooltip]} />
        <SeriesCollectionDirective>
          <SeriesDirective
            dataSource={horas}
            xName="etiqueta"
            yName="entreSemana"
            name="Lunes a viernes"
            type="Column"
            fill={estilo.serie[0]}
            cornerRadius={esquinas}
            columnSpacing={0.15}
            border={{ width: 0 }}
          />
          <SeriesDirective
            dataSource={horas}
            xName="etiqueta"
            yName="finDeSemana"
            name="Sábado y domingo"
            type="Column"
            fill={estilo.serie[1]}
            cornerRadius={esquinas}
            columnSpacing={0.15}
            border={{ width: 0 }}
          />
        </SeriesCollectionDirective>
      </ChartComponent>
    </Tarjeta>
  );
}

const EMPRESAS_EN_GRAFICO = 10;

/** Empresas con más ingresos del período, de mayor a menor (barras
 * horizontales: los nombres son largos). Una sola serie, sin leyenda. */
export function GraficoEmpresas({ empresas }: { empresas: ResumenEmpresa[] }) {
  const estilo = useEstilo();
  // El eje de categorías dibuja de abajo hacia arriba: se invierte para que la
  // primera quede arriba.
  const datos = empresas
    .slice(0, EMPRESAS_EN_GRAFICO)
    .map((e) => ({ ...e, etiqueta: e.empresa.length > 28 ? `${e.empresa.slice(0, 27)}…` : e.empresa }))
    .reverse();
  return (
    <Tarjeta
      titulo="Empresas con más ingresos"
      detalle={`Las ${EMPRESAS_EN_GRAFICO} primeras del período; el detalle está en la tabla dinámica.`}
    >
      <ChartComponent
        id="grafico-empresas"
        locale={CULTURA}
        height={ALTO}
        background="transparent"
        enableHtmlSanitizer
        chartArea={AREA}
        primaryXAxis={ejeX(estilo, { valueType: "Category", interval: 1 })}
        primaryYAxis={ejeY(estilo)}
        legendSettings={{ visible: false }}
        tooltip={tooltip(estilo, {
          format: "${point.tooltip}",
          header: "",
        })}
      >
        <Inject services={[BarSeries, Category, Tooltip]} />
        <SeriesCollectionDirective>
          <SeriesDirective
            dataSource={datos.map((d) => ({
              ...d,
              tooltip: `${escaparHtml(d.empresa)}<br/>Ingresos: <b>${d.ingresos.toLocaleString(CULTURA)}</b><br/>Personas: <b>${d.personas.toLocaleString(CULTURA)}</b>`,
            }))}
            xName="etiqueta"
            yName="ingresos"
            tooltipMappingName="tooltip"
            name="Ingresos"
            type="Bar"
            fill={estilo.serie[0]}
            cornerRadius={{ topLeft: 0, bottomLeft: 0, topRight: 4, bottomRight: 4 }}
            columnWidth={0.6}
            border={{ width: 0 }}
          />
        </SeriesCollectionDirective>
      </ChartComponent>
    </Tarjeta>
  );
}
