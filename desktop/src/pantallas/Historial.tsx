import { useCallback, useMemo, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { FileSpreadsheet, FileText } from "lucide-react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import type { TablaHandle } from "../componentes/Tabla";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import SelectorRangoFecha, { textoRangoFecha } from "../componentes/SelectorRangoFecha";
import {
  exportarHistorial,
  exportarHistorialPdf,
  listarHistorial,
  listarHistorialSitio,
  medioIngresoDesdeNube,
  textoMedioConPlaca,
  tipoIngresoDesdeNube,
} from "../api";
import type { MovimientoHistorialRemoto, MovimientoIngresoResumen } from "../api";
import { fechaHaceMeses, fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

/** Local (este dispositivo, con la auditoría completa de la decisión de
 * acceso) o remota (generada por otro dispositivo del mismo sitio, leída
 * de la caché `historial_sitio` -- ver `docs/planes-implementados/plan-persistencia-nube.md`).
 * Decisión explícita del usuario: es la misma operación vista desde otro
 * dispositivo, no una versión resumida -- se combinan en una sola grilla
 * con los mismos campos que ya muestra Historial. Excel/PDF recortan por
 * `uuid` (ver `seleccionParaExportar`), que ambas tienen -- una fila remota
 * no tiene `registro_id` local. */
interface FilaLocal extends MovimientoIngresoResumen {
  origen: "local";
  // Siempre "pc": esta pantalla sólo existe en el build de escritorio, no
  // hace falta leerlo de ningún lado -- a diferencia de una fila remota,
  // que sí puede venir de cualquier tipo de dispositivo del sitio.
  dispositivo_tipo: "pc";
}

interface FilaRemota {
  origen: "remoto";
  uuid: string;
  registro_id: null;
  contratista_id: null;
  cedula: string | null;
  contratista_nombre: string;
  empresa_nombre: string | null;
  tipo_ingreso: MovimientoIngresoResumen["tipo_ingreso"] | null;
  medio_ingreso: MovimientoIngresoResumen["medio_ingreso"] | null;
  fecha_hora_ingreso: string;
  fecha_hora_salida: string | null;
  gafete_numero: number | null;
  placa: string | null;
  usuario_ingreso_nombre: string;
  usuario_salida_nombre: string | null;
  resultado_acceso: null;
  motivo_resultado: null;
  reglas_version: null;
  empresa_activa_snapshot: null;
  // `null` para movimientos remotos sincronizados antes de que
  // `historial_sitio.dispositivo_entrada_tipo` existiera (migración 26).
  dispositivo_tipo: string | null;
}

type FilaHistorial = FilaLocal | FilaRemota;

/** "pc"/"mobile" (`dispositivos.tipo`) → sólo el ícono para la columna
 * "Dispositivo" -- ícono+palabra ("💻 PC"/"📱 Celular") quedaba desparejo
 * visualmente (una palabra bastante más larga que la otra). Cualquier otro
 * valor (o `null`) se muestra tal cual / como "—", nunca se inventa un tipo
 * que no vino. */
export function textoDispositivo(tipo: string | null): string {
  if (tipo === "pc") return "💻";
  if (tipo === "mobile") return "📱";
  return tipo ?? "—";
}

/** `historial_sitio` incluye a propósito los movimientos de ESTE mismo
 * dispositivo (respaldo ante una reinstalación que pierda
 * `registro_ingresos` local, ver `src/nube/sincronizacion.rs`,
 * `recibir_historial_del_sitio`) -- sin este filtro, cada movimiento que ya
 * se sincronizó aparece dos veces: una vez como local y otra como remoto
 * (bug real en producción, 2026-09-11). Mismo criterio que ya aplica la UI
 * de Android (`HistorialViewModel.kt`). */
export function remotosSinDuplicarLocales(
  locales: readonly { uuid: string }[],
  remotos: readonly MovimientoHistorialRemoto[],
): MovimientoHistorialRemoto[] {
  const localesUuids = new Set(locales.map((fila) => fila.uuid));
  return remotos.filter((remoto) => !localesUuids.has(remoto.uuid));
}

export function filaDesdeRemoto(remoto: MovimientoHistorialRemoto): FilaHistorial {
  return {
    origen: "remoto",
    uuid: remoto.uuid,
    registro_id: null,
    contratista_id: null,
    cedula: remoto.cedula,
    contratista_nombre: remoto.contratista_nombre,
    empresa_nombre: remoto.empresa_nombre,
    tipo_ingreso: tipoIngresoDesdeNube(remoto.tipo_ingreso),
    medio_ingreso: medioIngresoDesdeNube(remoto.medio_ingreso),
    fecha_hora_ingreso: remoto.fecha_hora_ingreso,
    fecha_hora_salida: remoto.fecha_hora_salida,
    gafete_numero: remoto.gafete_numero,
    placa: remoto.placa,
    usuario_ingreso_nombre: remoto.usuario_ingreso_nombre ?? "—",
    usuario_salida_nombre: remoto.usuario_salida_nombre,
    resultado_acceso: null,
    motivo_resultado: null,
    reglas_version: null,
    empresa_activa_snapshot: null,
    dispositivo_tipo: remoto.dispositivo_entrada_tipo,
  };
}

/** colId/field de la grilla → clave de `ColumnaHistorial` en el núcleo
 * (`src/historial/exportacion.rs`, `ColumnaHistorial::clave`) — así el
 * export sabe qué columnas del XLSX corresponden a las que el usuario
 * dejó visibles acá. "Fecha salida" no tiene equivalente propio en el
 * núcleo salvo `fecha_salida` (agregado junto con esta columna); ambas
 * hoy están 1 a 1. */
const CLAVES_COLUMNA: Record<string, string> = {
  cedula: "cedula",
  contratista_nombre: "nombre",
  empresa_nombre: "empresa",
  tipo_ingreso: "tipo",
  medio_ingreso: "medio",
  gafete_numero: "gafete",
  fecha_ingreso: "fecha",
  hora_ingreso: "entrada",
  fecha_salida: "fecha_salida",
  hora_salida: "salida",
  usuario_ingreso_nombre: "ingreso",
  usuario_salida_nombre: "egreso",
};

/** Identidad de fila para el destello de celdas cambiadas (`idFila` de
 * `Tabla`) -- a nivel de módulo para que sea una función estable. */
const idPorUuid = (fila: { uuid: string }) => fila.uuid;

function totalesHistorial(visibles: FilaHistorial[]): Record<string, string> {
  return { contratista_nombre: `${visibles.length} movimiento(s)` };
}

export default function Historial() {
  const [filas, setFilas] = useState<FilaHistorial[]>([]);
  const [cargando, setCargando] = useState(true);
  const [exportando, setExportando] = useState(false);
  // `true` cuando el rango actual supera el tope de carga completa del
  // núcleo (`LIMITE_CARGA_COMPLETA_MAXIMO`, `CargaCompleta.truncado`) — la
  // grilla sigue funcionando exactamente igual (client-side, filtro por
  // columna instantáneo) sobre las filas que sí trajo, pero el filtro por
  // columna y "exportar lo visible" dejan de representar el rango
  // completo. Mientras el total esté bajo el tope (el caso normal) nada de
  // esto se activa — un solo camino de código, sin modo aparte que
  // mantener.
  const [truncado, setTruncado] = useState(false);
  // Por defecto trae los últimos 6 meses — antes traía todo desde el año
  // 2000 (rango fijo en el backend). `hasta` vacío queda abierto (hoy + 1
  // día en el backend, ver `rango_utc`), así no se pierden movimientos del
  // día en curso. El usuario puede ampliar `desde` para ver más atrás.
  const [desde, setDesde] = useState(() => fechaHaceMeses(6));
  const [hasta, setHasta] = useState("");
  const tablaRef = useRef<TablaHandle<FilaHistorial>>(null);

  useBarraEstado(
    cargando
      ? "Cargando…"
      : truncado
        ? `${filas.length}+ movimiento(s) (rango truncado)`
        : `${filas.length} movimiento(s)`,
  );

  const recargar = useCallback(
    async (estaVigente: () => boolean = () => true) => {
      setCargando(true);
      try {
        const [{ items, truncado }, remotos] = await Promise.all([
          listarHistorial(desde || undefined, hasta || undefined),
          listarHistorialSitio(desde || undefined, hasta || undefined),
        ]);
        if (!estaVigente()) return;
        const locales: FilaHistorial[] = items.map((item) => ({
          ...item,
          origen: "local",
          dispositivo_tipo: "pc",
        }));
        const remotosFiltrados = remotosSinDuplicarLocales(locales, remotos);
        setFilas([...locales, ...remotosFiltrados.map(filaDesdeRemoto)]);
        setTruncado(truncado);
      } finally {
        if (estaVigente()) setCargando(false);
      }
    },
    [desde, hasta],
  );
  useCargaAlCambiar(recargar, true);

  // Lo que la grilla tiene visible AHORA (filtro por columna y selector
  // "Columnas ▾" de AG Grid, ambos del lado del cliente) — Excel y PDF
  // exportan ese mismo recorte en vez de siempre mandar todo el historial
  // sin acotar. Devuelve `null` (con el toast de error ya disparado) si no
  // hay nada exportable, para que quien llama corte ahí sin duplicar el
  // chequeo. Cuando `truncado` es `true`, el cliente sólo tiene una
  // porción del rango — `uuids: null` le dice al backend que exporte todo
  // `desde`/`hasta` directo de la base en vez de la porción cargada (ver
  // `exportarHistorial`/`exportarHistorialPdf`); el filtro por columna deja
  // de aplicar ahí porque ya no puede evaluarse sobre el total real.
  function seleccionParaExportar(): { uuids: string[] | null; claves: string[] } | null {
    const claves = (tablaRef.current?.columnasVisibles() ?? Object.keys(CLAVES_COLUMNA))
      .map((colId) => CLAVES_COLUMNA[colId])
      .filter((clave): clave is string => clave !== undefined);
    if (claves.length === 0) {
      toast.error("No hay columnas visibles para exportar.");
      return null;
    }
    if (truncado) return { uuids: null, claves };

    const visibles = tablaRef.current?.filasFiltradas() ?? filas;
    if (visibles.length === 0) {
      toast.error("No hay filas para exportar con el filtro actual.");
      return null;
    }
    return { uuids: visibles.map((fila) => fila.uuid), claves };
  }

  async function exportar() {
    const seleccion = seleccionParaExportar();
    if (!seleccion) return;

    const destino = await save({
      title: "Exportar historial a Excel",
      defaultPath: "historial.xlsx",
      filters: [{ name: "Excel", extensions: ["xlsx"] }],
    });
    if (!destino) return;

    setExportando(true);
    toast.promise(
      exportarHistorial(
        destino,
        seleccion.uuids,
        seleccion.claves,
        desde || undefined,
        hasta || undefined,
      ).finally(() => setExportando(false)),
      {
        loading: "Exportando…",
        success: (cantidad) => `${cantidad} fila(s) exportadas.`,
        error: (error) => String(error),
      },
    );
  }

  async function exportarPdf() {
    const seleccion = seleccionParaExportar();
    if (!seleccion) return;

    const destino = await save({
      title: "Exportar historial a PDF",
      defaultPath: "historial.pdf",
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (!destino) return;

    setExportando(true);
    toast.promise(
      exportarHistorialPdf(
        destino,
        seleccion.uuids,
        seleccion.claves,
        `Filtro: ${textoRangoFecha(desde, hasta)}`,
        desde || undefined,
        hasta || undefined,
      ).finally(() => setExportando(false)),
      {
        loading: "Exportando…",
        success: "PDF exportado.",
        error: (error) => String(error),
      },
    );
  }

  // useMemo a propósito — mismo motivo que Activos.tsx: si `columnas` se
  // recrea en cada render, AG Grid reaplica el orden/ancho literales de acá
  // encima del layout que el usuario ya acomodó (persistido en localStorage
  // vía `Tabla`).
  const columnas: ColDef<FilaHistorial>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", flex: 1.2, minWidth: 120, cellStyle: { textAlign: "left" } },
      {
        field: "contratista_nombre",
        headerName: "Nombre",
        flex: 1.4,
        minWidth: 160,
        cellStyle: { textAlign: "left" },
      },
      { field: "empresa_nombre", headerName: "Empresa", flex: 1, minWidth: 130 },
      {
        field: "dispositivo_tipo",
        headerName: "Dispositivo",
        flex: 1,
        minWidth: 110,
        valueFormatter: (p) => textoDispositivo(p.value ?? null),
        // Buscador de arriba (quickFilter) limitado a Cédula/Nombre/Empresa
        // -- las tres cosas que identifican a LA PERSONA que se busca, mismo
        // criterio que Activos.tsx (hallazgo real del usuario 2026-09-21:
        // buscar "daniel" traía en cambio quien dio el ingreso). Quien
        // necesite filtrar por el resto de estas columnas ya tiene el
        // filtro propio de cada columna (`floatingFilter`), sin tocar.
        getQuickFilterText: () => "",
      },
      {
        field: "tipo_ingreso",
        headerName: "Tipo",
        flex: 1,
        minWidth: 100,
        getQuickFilterText: () => "",
      },
      {
        field: "medio_ingreso",
        headerName: "Medio",
        flex: 1,
        minWidth: 100,
        valueFormatter: (p) => textoMedioConPlaca(p.value, p.data?.placa ?? null),
        getQuickFilterText: () => "",
      },
      {
        field: "gafete_numero",
        type: "numero",
        headerName: "Gafete",
        flex: 0.9,
        minWidth: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
        getQuickFilterText: () => "",
      },
      {
        colId: "fecha_ingreso",
        type: "fecha",
        headerName: "Fecha ingreso",
        // 120 truncaba el título en mayúscula ("FECHA ING…") mientras el
        // resto de encabezados entraba completo — 140 es lo que necesita
        // "FECHA INGRESO" para no cortarse.
        flex: 1.4,
        minWidth: 140,
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_ingreso) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
        getQuickFilterText: () => "",
      },
      {
        colId: "hora_ingreso",
        headerName: "Hora ingreso",
        flex: 1.3,
        minWidth: 130,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_ingreso) : ""),
        getQuickFilterText: () => "",
      },
      {
        // "Activo" (no sólo vacío) cuando no hay salida — mismo texto que
        // ya escribe la exportación a Excel (`escribir_movimiento`,
        // `src/historial/exportacion.rs`) para un movimiento sin
        // `fecha_hora_salida`; antes la grilla dejaba la celda en blanco y
        // no coincidía con lo que se veía en el archivo exportado.
        colId: "fecha_salida",
        type: "fecha",
        headerName: "Fecha salida",
        flex: 1.4,
        minWidth: 140,
        valueGetter: (p) =>
          p.data?.fecha_hora_salida ? fechaLocalYMD(p.data.fecha_hora_salida) : "Activo",
        valueFormatter: (p) => (p.value === "Activo" ? "Activo" : textoFechaDDMMYYYY(p.value)),
        getQuickFilterText: () => "",
      },
      {
        colId: "hora_salida",
        headerName: "Hora salida",
        flex: 1.3,
        minWidth: 130,
        valueGetter: (p) =>
          p.data?.fecha_hora_salida ? textoHora(p.data.fecha_hora_salida) : "Activo",
        getQuickFilterText: () => "",
      },
      {
        field: "usuario_ingreso_nombre",
        headerName: "Dio ingreso",
        flex: 1.3,
        minWidth: 130,
        getQuickFilterText: () => "",
      },
      {
        field: "usuario_salida_nombre",
        headerName: "Dio salida",
        flex: 1.3,
        minWidth: 130,
        getQuickFilterText: () => "",
      },
    ],
    [],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        {truncado && (
          <p
            role="status"
            style={{
              margin: "0 0 0.5rem",
              padding: "0.5rem 0.75rem",
              borderRadius: "var(--radio-chico)",
              border: "1px solid var(--advertencia)",
              color: "var(--advertencia)",
              fontSize: "0.85rem",
            }}
          >
            Este rango tiene más de {filas.length.toLocaleString("es-CR")} movimientos — se
            muestran solo los primeros. El filtro por columna sólo aplica a lo cargado; acotá las
            fechas para verlo todo, o exportá igual: Excel y PDF traen el rango completo aunque no
            esté cargado en pantalla.
          </p>
        )}
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<FilaHistorial>
            cargando={cargando}
            ref={tablaRef}
            id="historial"
            idFila={idPorUuid}
            filaTotales={totalesHistorial}
            columnas={columnas}
            filas={filas}
            filtrosPorColumna
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
                  title={
                    exportando
                      ? "Exportando…"
                      : truncado
                        ? "Exportar a Excel — trae todo el rango de fechas, no sólo lo cargado"
                        : "Exportar a Excel — respeta el filtro/orden/columnas actuales de la grilla"
                  }
                  onClick={exportar}
                  disabled={exportando}
                >
                  <FileSpreadsheet size={16} />
                </button>
                <button
                  type="button"
                  className="boton boton-icono"
                  title={
                    exportando
                      ? "Exportando…"
                      : truncado
                        ? "Exportar a PDF — trae todo el rango de fechas, no sólo lo cargado"
                        : "Exportar a PDF — respeta el filtro/orden/columnas actuales de la grilla"
                  }
                  onClick={exportarPdf}
                  disabled={exportando}
                >
                  <FileText size={16} />
                </button>
              </>
            }
          />
        </div>
      </div>
    </div>
  );
}
