import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { FileSpreadsheet } from "lucide-react";
import * as XLSX from "xlsx";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import type { TablaHandle } from "../componentes/Tabla";
import SelectorRangoFecha from "../componentes/SelectorRangoFecha";
import { useAutoRefresh } from "../componentes/useAutoRefresh";
import { listarHistorial } from "../api/historial";
import type { MovimientoHistorial } from "../api/historial";
import { fechaHaceMeses, fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

function textoMedio(medio: string | null): string {
  if (medio === "CAMINANDO") return "Caminando";
  if (medio === "VEHICULO") return "Vehículo";
  return "";
}

/**
 * Historial multi-sitio para `admin_global` -- lee `ingresos` en Supabase
 * (ver `api/historial.ts` y la migración `agrega_columnas_historial_a_ingresos`),
 * no la base local de un sitio en particular como la versión de escritorio.
 * "Exportar a Excel" es client-side (SheetJS) en vez del exportador de
 * Rust/Tauri de `desktop/` -- este panel no tiene un proceso nativo del
 * lado del navegador que escriba el archivo.
 */
export default function Historial() {
  const [filas, setFilas] = useState<MovimientoHistorial[]>([]);
  const [cargando, setCargando] = useState(true);
  const [busqueda, setBusqueda] = useState("");
  // Mismo default que desktop/src/pantallas/Historial.tsx -- últimos 6
  // meses, `hasta` abierto para no perderse movimientos del día en curso.
  const [desde, setDesde] = useState(() => fechaHaceMeses(6));
  const [hasta, setHasta] = useState("");
  const tablaRef = useRef<TablaHandle<MovimientoHistorial>>(null);

  const recargar = useCallback((opciones?: { silencioso?: boolean }) => {
    const silencioso = opciones?.silencioso ?? false;
    if (!silencioso) setCargando(true);
    return listarHistorial(desde || undefined, hasta || undefined)
      .then(setFilas)
      .catch((error) => {
        if (!silencioso) toast.error(String(error));
      })
      .finally(() => {
        if (!silencioso) setCargando(false);
      });
  }, [desde, hasta]);

  useEffect(() => {
    recargar();
  }, [recargar]);

  // Ver `useAutoRefresh` -- sin esto, un ingreso ya cerrado/sincronizado
  // no aparecía hasta apretar "actualizar" a mano.
  useAutoRefresh(() => recargar({ silencioso: true }), 30_000, "ingresos");

  function exportarAExcel() {
    const visibles = tablaRef.current?.filasFiltradas() ?? filas;
    if (visibles.length === 0) {
      toast.error("No hay filas para exportar con el filtro actual.");
      return;
    }

    const datos = visibles.map((fila) => ({
      Sitio: fila.sitio_nombre ?? "",
      Cédula: fila.contratista_cedula ?? "",
      Nombre: fila.contratista_nombre,
      Empresa: fila.empresa_nombre ?? "",
      Tipo: fila.tipo_ingreso ?? "",
      Medio: textoMedio(fila.medio_ingreso),
      Gafete: fila.gafete_numero ?? "S/G",
      "Fecha ingreso": textoFechaDDMMYYYY(fechaLocalYMD(fila.hora_entrada)),
      "Hora ingreso": textoHora(fila.hora_entrada),
      "Fecha salida": fila.hora_salida ? textoFechaDDMMYYYY(fechaLocalYMD(fila.hora_salida)) : "Activo",
      "Hora salida": fila.hora_salida ? textoHora(fila.hora_salida) : "Activo",
      "Dio ingreso": fila.usuario_entrada_nombre ?? "",
      "Dio salida": fila.usuario_salida_nombre ?? "",
    }));

    const hoja = XLSX.utils.json_to_sheet(datos);
    const libro = XLSX.utils.book_new();
    XLSX.utils.book_append_sheet(libro, hoja, "Historial");
    XLSX.writeFile(libro, "historial.xlsx");
  }

  // useMemo -- mismo motivo que en desktop/: si `columnas` se recrea en
  // cada render, AG Grid reaplica el orden/ancho literales encima del
  // layout que el usuario ya acomodó (persistido en localStorage vía
  // `Tabla`).
  const columnas: ColDef<MovimientoHistorial>[] = useMemo(
    () => [
      { field: "sitio_nombre", headerName: "Sitio", flex: 1, minWidth: 110 },
      {
        field: "contratista_cedula",
        headerName: "Cédula",
        flex: 1.1,
        minWidth: 110,
        cellStyle: { textAlign: "left" },
      },
      {
        field: "contratista_nombre",
        headerName: "Nombre",
        flex: 1.4,
        minWidth: 160,
        cellStyle: { textAlign: "left" },
      },
      { field: "empresa_nombre", headerName: "Empresa", flex: 1, minWidth: 130 },
      { field: "tipo_ingreso", headerName: "Tipo", flex: 1, minWidth: 100 },
      {
        field: "medio_ingreso",
        headerName: "Medio",
        flex: 1,
        minWidth: 100,
        valueFormatter: (p) => textoMedio(p.value),
      },
      {
        field: "gafete_numero",
        headerName: "Gafete",
        flex: 0.9,
        minWidth: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
      {
        colId: "fecha_ingreso",
        headerName: "Fecha ingreso",
        flex: 1.4,
        minWidth: 140,
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.hora_entrada) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        colId: "hora_ingreso",
        headerName: "Hora ingreso",
        flex: 1.3,
        minWidth: 130,
        valueGetter: (p) => (p.data ? textoHora(p.data.hora_entrada) : ""),
      },
      {
        colId: "fecha_salida",
        headerName: "Fecha salida",
        flex: 1.4,
        minWidth: 140,
        valueGetter: (p) => (p.data?.hora_salida ? fechaLocalYMD(p.data.hora_salida) : "Activo"),
        valueFormatter: (p) => (p.value === "Activo" ? "Activo" : textoFechaDDMMYYYY(p.value)),
      },
      {
        colId: "hora_salida",
        headerName: "Hora salida",
        flex: 1.3,
        minWidth: 130,
        valueGetter: (p) => (p.data?.hora_salida ? textoHora(p.data.hora_salida) : "Activo"),
      },
      { field: "usuario_entrada_nombre", headerName: "Dio ingreso", flex: 1.3, minWidth: 130 },
      { field: "usuario_salida_nombre", headerName: "Dio salida", flex: 1.3, minWidth: 130 },
    ],
    [],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<MovimientoHistorial>
            ref={tablaRef}
            id="historial"
            columnas={columnas}
            filas={filas}
            filtrosPorColumna
            busqueda={busqueda}
            controles={
              <div className="campo" style={{ flex: "0 1 16rem" }}>
                <input
                  placeholder="Cédula, nombre, empresa…"
                  value={busqueda}
                  onChange={(evento) => setBusqueda(evento.target.value)}
                />
              </div>
            }
            accionesDerecha={
              <>
                <SelectorRangoFecha
                  desde={desde}
                  hasta={hasta}
                  onAplicar={(nuevoDesde, nuevoHasta) => {
                    setDesde(nuevoDesde);
                    setHasta(nuevoHasta);
                  }}
                />
                <button
                  type="button"
                  className="boton boton-icono"
                  title="Exportar a Excel — respeta el filtro/orden/columnas actuales de la grilla"
                  onClick={exportarAExcel}
                  disabled={cargando}
                >
                  <FileSpreadsheet size={16} />
                </button>
              </>
            }
          />
        </div>
      </div>
    </div>
  );
}
