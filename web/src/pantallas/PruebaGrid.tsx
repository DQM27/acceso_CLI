import { useEffect, useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  ColumnChooser,
  ColumnDirective,
  ColumnsDirective,
  ExcelExport,
  Filter,
  GridComponent,
  Group,
  Inject,
  Reorder,
  Resize,
  Search,
  Sort,
  Toolbar,
  VirtualScroll,
} from "@syncfusion/ej2-react-grids";
import type { ToolbarItems } from "@syncfusion/ej2-react-grids";
import { listarMovimientosParaExportar } from "../api/historial";
import { fechaHaceMeses } from "../tiempo";
import { CULTURA } from "./analisis/syncfusion";

/**
 * PRUEBA, no es una pantalla definitiva: el historial de movimientos en una
 * grilla de Syncfusion, para compararla con AG Grid. Muestra lo que trae de
 * fábrica: búsqueda, orden por varias columnas, filtros tipo Excel, agrupar
 * arrastrando una columna a la franja de arriba, mover y redimensionar
 * columnas, elegir columnas visibles y exportar a Excel.
 *
 * Carga todo el rango de una vez (hasta `MAXIMO_FILAS`) y filtra en el
 * navegador, a diferencia de Historial, que pagina en el servidor.
 */
const BARRA: ToolbarItems[] = ["Search", "ColumnChooser", "ExcelExport"];
const MESES = 6;
const MAXIMO_FILAS = 10_000;

function minutosEntre(
  desdeIso: string,
  hastaIso: string | null,
): number | null {
  if (!hastaIso) return null;
  return Math.max(
    0,
    Math.round(
      (new Date(hastaIso).getTime() - new Date(desdeIso).getTime()) / 60_000,
    ),
  );
}

export default function PruebaGrid() {
  const { data, isFetching } = useQuery({
    queryKey: ["prueba-grid", "historial", MESES],
    queryFn: () =>
      listarMovimientosParaExportar(
        { desde: fechaHaceMeses(MESES) },
        MAXIMO_FILAS,
      ),
    staleTime: 60_000,
  });
  const grilla = useRef<GridComponent>(null);

  // Con virtualización la grilla calcula cuántas filas dibujar a partir de su
  // alto; con `height="100%"` dentro de un contenedor flex lo calculaba mal y
  // dejaba un hueco al arrancar. Se le pasa el alto real en píxeles.
  const contenedor = useRef<HTMLDivElement>(null);
  const [alto, setAlto] = useState(0);
  useEffect(() => {
    const el = contenedor.current;
    if (!el) return;
    const observador = new ResizeObserver(() => setAlto(el.clientHeight));
    observador.observe(el);
    setAlto(el.clientHeight);
    return () => observador.disconnect();
  }, []);

  // La grilla trabaja con campos planos y fechas reales (para ordenar y filtrar
  // por fecha); el texto de pantalla de cada fila ya viene de la vista.
  const filas = useMemo(
    () =>
      (data?.filas ?? []).map((m) => ({
        id: m.id,
        unidad: m.sitio_nombre ?? "",
        cedula: m.contratista_cedula ?? "",
        nombre: m.contratista_nombre,
        empresa: m.empresa_nombre ?? "",
        tipo: m.tipo_texto,
        medio: m.medio_texto,
        gafete: m.gafete_numero,
        entrada: new Date(m.hora_entrada),
        salida: m.hora_salida ? new Date(m.hora_salida) : null,
        minutos: minutosEntre(m.hora_entrada, m.hora_salida),
        usuario_entrada: m.usuario_entrada_nombre ?? "",
        usuario_salida: m.usuario_salida_nombre ?? "",
        dispositivo: m.dispositivo_entrada_tipo ?? "",
      })),
    [data],
  );

  function alClicBarra(args: { item: { id: string } }) {
    if (args.item.id.endsWith("_excelexport"))
      grilla.current?.excelExport({ fileName: "historial.xlsx" });
  }

  return (
    <div className="flex h-full flex-col">
      <div className="pantalla-cuerpo min-h-0 flex-1">
        <p className="text-xs text-(--muted)" role="status">
          {isFetching && !data
            ? "Cargando historial…"
            : `Prueba de la grilla de Syncfusion: ${filas.length.toLocaleString("es-CR")} movimientos de los últimos ${MESES} meses` +
              (data?.truncado
                ? ` (tope de ${MAXIMO_FILAS.toLocaleString("es-CR")}, hay más)`
                : "") +
              ". Arrastre un encabezado a la franja de arriba para agrupar; el menú de cada columna filtra y ordena."}
        </p>
        <div ref={contenedor} className="min-h-0 flex-1">
          <GridComponent
            ref={grilla}
            dataSource={filas}
            locale={CULTURA}
            height={alto || "100%"}
            pageSettings={{ pageSize: 60 }}
            width="100%"
            enableVirtualization
            allowSorting
            allowMultiSorting
            allowFiltering
            allowGrouping
            allowResizing
            allowReordering
            allowExcelExport
            showColumnChooser
            filterSettings={{ type: "Excel" }}
            groupSettings={{ showDropArea: true }}
            toolbar={BARRA}
            toolbarClick={alClicBarra}
          >
            <ColumnsDirective>
              <ColumnDirective
                field="id"
                isPrimaryKey
                visible={false}
                showInColumnChooser={false}
              />
              <ColumnDirective field="unidad" headerText="Unidad" width={140} />
              <ColumnDirective field="cedula" headerText="Cédula" width={130} />
              <ColumnDirective field="nombre" headerText="Nombre" width={200} />
              <ColumnDirective
                field="empresa"
                headerText="Empresa"
                width={170}
              />
              <ColumnDirective
                field="tipo"
                headerText="Tipo de ingreso"
                width={170}
              />
              <ColumnDirective field="medio" headerText="Medio" width={110} />
              <ColumnDirective
                field="gafete"
                headerText="Gafete"
                width={100}
                type="number"
                format="n0"
                textAlign="Right"
              />
              <ColumnDirective
                field="entrada"
                headerText="Entrada"
                width={160}
                type="datetime"
                format="dd/MM/yyyy HH:mm"
              />
              <ColumnDirective
                field="salida"
                headerText="Salida"
                width={160}
                type="datetime"
                format="dd/MM/yyyy HH:mm"
              />
              <ColumnDirective
                field="minutos"
                headerText="Minutos adentro"
                width={160}
                type="number"
                format="n0"
                textAlign="Right"
              />
              <ColumnDirective
                field="usuario_entrada"
                headerText="Dio ingreso"
                width={150}
              />
              <ColumnDirective
                field="usuario_salida"
                headerText="Dio salida"
                width={150}
              />
              <ColumnDirective
                field="dispositivo"
                headerText="Dispositivo"
                width={110}
              />
            </ColumnsDirective>
            <Inject
              services={[
                Sort,
                Filter,
                Group,
                Resize,
                Reorder,
                ColumnChooser,
                Toolbar,
                Search,
                ExcelExport,
                VirtualScroll,
              ]}
            />
          </GridComponent>
        </div>
      </div>
    </div>
  );
}
