import { useCallback, useEffect, useMemo, useState } from "react";
import { avisos } from "../avisos/avisos";
import { CircleOff, IdCard, LogIn, LogOut, Users } from "lucide-react";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import Modal from "../componentes/Modal";
import { useAccionBarraEstado } from "../contexto/BarraEstadoContexto";
import {
  cerrarFilaActiva,
  claveFilaActiva,
  listarTodosLosActivos,
  textoMedioConPlaca,
  textoTipoIngreso,
} from "../api";
import type { FilaActiva } from "../api";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import { contarPorGafete, filtrarPorGafete, masDeDoceHoras } from "./Activos.logica";
import type { FiltroGafete } from "./Activos.logica";

/** Ícono, número y nombre completo (al pasar el mouse) de cada opción. Los
 * números son el conteo de gente adentro: el total, y cuántos tienen gafete y
 * cuántos no (S/G). Mientras carga no se muestran, para no enseñar ceros
 * falsos. */
function opcionesFiltroGafete(
  conteo: { todos: number; con: number; sin: number } | null,
): OpcionSegmentada<FiltroGafete>[] {
  return [
    {
      valor: "todos",
      Icono: Users,
      titulo: conteo ? `${conteo.todos} adentro` : "Todos",
      cantidad: conteo?.todos,
    },
    {
      valor: "con",
      Icono: IdCard,
      titulo: conteo ? `${conteo.con} con gafete` : "Con gafete",
      cantidad: conteo?.con,
    },
    {
      valor: "sin",
      Icono: CircleOff,
      titulo: conteo ? `${conteo.sin} sin gafete (S/G)` : "Sin gafete (S/G)",
      cantidad: conteo?.sin,
    },
  ];
}

/** Pieza única de íconos con el relleno deslizante (`SegmentadoOpciones`). */
function ToggleGafete({
  filtro,
  conteo,
  onCambiar,
}: {
  filtro: FiltroGafete;
  conteo: { todos: number; con: number; sin: number } | null;
  onCambiar: (filtro: FiltroGafete) => void;
}) {
  return (
    <SegmentadoOpciones
      opciones={opcionesFiltroGafete(conteo)}
      valor={filtro}
      onCambiar={onCambiar}
      etiqueta="Filtrar por gafete"
    />
  );
}

function claseFilaActiva(fila: FilaActiva): string | undefined {
  return masDeDoceHoras(fila) ? "fila-mas-de-12-horas" : undefined;
}

export default function Activos({
  refrescarSenal,
  onAbrirNuevoIngreso,
  onAbrirSalida,
}: {
  /** Los modales de Nuevo Ingreso y Salida viven en el Shell (se disparan
   * desde cualquier pantalla vía Ctrl+N/S, no sólo desde acá) —
   * este número sube cada vez que registran algo, para que la grilla se
   * refresque aunque ya estuviera montada. */
  refrescarSenal?: number;
  onAbrirNuevoIngreso: () => void;
  onAbrirSalida: () => void;
}) {
  const [filas, setFilas] = useState<FilaActiva[]>([]);
  const [total, setTotal] = useState(0);
  const [cargando, setCargando] = useState(true);
  // Los números del filtro sólo se ocultan en la PRIMERA carga: en las
  // siguientes (cada ingreso o salida recarga) se quedan los anteriores hasta
  // que llegan los nuevos, en vez de parpadear.
  const [cargadoAlgunaVez, setCargadoAlgunaVez] = useState(false);
  const [busqueda, setBusqueda] = useState("");
  const [seleccionadas, setSeleccionadas] = useState<FilaActiva[]>([]);
  const [confirmarSalidaMasiva, setConfirmarSalidaMasiva] = useState(false);
  const [procesando, setProcesando] = useState(false);
  const [filtroGafete, setFiltroGafete] = useState<FiltroGafete>("todos");
  const filasVisibles = useMemo(() => filtrarPorGafete(filas, filtroGafete), [filas, filtroGafete]);
  // Los números de los botones del filtro (ver `opcionesFiltroGafete`): son la
  // única métrica de la pantalla, ya no hay texto en la barra de estado.
  const conteo = useMemo(
    () => (cargadoAlgunaVez ? { todos: total, ...contarPorGafete(filas) } : null),
    [cargadoAlgunaVez, total, filas],
  );

  // En la barra de estado, no sobre la grilla: antes una franja con
  // "N seleccionado(s)" aparecía arriba y corría toda la pantalla hacia
  // abajo (pedido del usuario 2026-09-23).
  useAccionBarraEstado(
    seleccionadas.length > 0 ? `Registrar salida (${seleccionadas.length})` : null,
    () => setConfirmarSalidaMasiva(true),
  );

  const recargar = useCallback(() => {
    // `Promise.resolve().then(...)` en vez de llamar `setCargando(true)`
    // directo -- de lo contrario `react-hooks/set-state-in-effect` marca
    // esta actualización de estado como síncrona dentro del cuerpo del
    // efecto que la dispara (abajo). Diferirla a un microtask no cambia
    // nada perceptible (corre antes del próximo paint igual) y cumple la
    // regla.
    return Promise.resolve()
      .then(() => setCargando(true))
      .then(() => listarTodosLosActivos())
      .then(({ filas, total }) => {
        setFilas(filas);
        setTotal(total);
        setCargadoAlgunaVez(true);
        setSeleccionadas([]);
      })
      .finally(() => setCargando(false));
  }, []);

  useEffect(() => {
    let vigente = true;
    recargar().catch((error) => vigente && avisos.error(error));
    return () => {
      vigente = false;
    };
  }, [recargar, refrescarSenal]);

  // useCallback a propósito: esta función se cierra dentro de una celda de
  // `columnas` — sin identidad estable, `columnas` (memoizado más abajo)
  // se recrearía en cada render igual, y con eso AG Grid reasignaría el
  // orden/ancho originales del código encima de lo que el usuario acomodó
  // (ver el comentario junto a `columnas`).
  const salidaIndividual = useCallback(
    async (fila: FilaActiva) => {
      try {
        await cerrarFilaActiva(fila);
        recargar();
      } catch (error) {
        avisos.error(error);
      }
    },
    [recargar],
  );

  async function confirmarSalidasSeleccionadas() {
    setProcesando(true);
    try {
      for (const fila of seleccionadas) {
        await cerrarFilaActiva(fila);
      }
      setConfirmarSalidaMasiva(false);
      await recargar();
    } catch (error) {
      avisos.error(error);
    } finally {
      setProcesando(false);
    }
  }

  // useMemo a propósito: si `columnas` se recrea en cada render (ej. cada
  // vez que `recargar()` trae datos nuevos), AG Grid recibe un `columnDefs`
  // "nuevo" y reaplica el orden/ancho literales de acá encima de lo que el
  // usuario ya había acomodado a mano — el layout persistido en
  // `Tabla`/localStorage quedaba pisado en el siguiente refresco.
  const columnas: ColDef<FilaActiva>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", flex: 1.2, minWidth: 120, cellStyle: { textAlign: "left" } },
      {
        field: "contratista_nombre",
        headerName: "Nombre",
        flex: 1.6,
        minWidth: 170,
        cellStyle: { textAlign: "left" },
      },
      { field: "empresa_nombre", headerName: "Empresa", flex: 1.1, minWidth: 140 },
      {
        field: "tipo_ingreso",
        headerName: "Tipo",
        flex: 1,
        minWidth: 100,
        // `valueGetter` (no `valueFormatter`): el filtro por columna, el
        // orden y las exportaciones usan el mismo texto que se ve.
        valueGetter: (p) => textoTipoIngreso(p.data?.tipo_ingreso ?? null),
        // Buscador de arriba (quickFilter) limitado a Cédula/Nombre/Empresa
        // -- las tres cosas que identifican a LA PERSONA que se busca.
        // `getQuickFilterText: () => ""` saca esta columna de esa búsqueda
        // sin afectar el filtro de columna propio (`floatingFilter`), que
        // sigue funcionando normal. Mismo criterio en el resto de columnas
        // de abajo -- hallazgo real del usuario 2026-09-21: buscar "daniel"
        // (un contratista) traía en cambio el registro dado de alta por el
        // operador Daniel, porque el buscador también miraba "Dio ingreso".
        getQuickFilterText: () => "",
      },
      {
        field: "medio_ingreso",
        headerName: "Medio",
        flex: 1,
        minWidth: 100,
        valueGetter: (p) =>
          p.data?.medio_ingreso == null
            ? "—"
            : textoMedioConPlaca(p.data.medio_ingreso, p.data.placa ?? null),
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
        headerName: "Fecha",
        flex: 1.1,
        minWidth: 110,
        // `valueGetter` (no `field`) a propósito, igual que Hora: si la
        // columna lee el string ISO crudo, AG Grid la infiere como fecha y
        // el filtro flotante termina siendo el selector nativo de
        // fecha+hora del navegador (`datetime-local`) en vez de un texto
        // simple — igual que le pasaba a Hora. `fechaLocalYMD` mantiene el
        // orden cronológico correcto como texto ("2026-08-28" ordena bien
        // sin volver a pasar por `Date`); `valueFormatter` sólo reacomoda
        // ese mismo string a DD/MM/AAAA para mostrar, sin re-parsearlo.
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_ingreso) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
        getQuickFilterText: () => "",
      },
      {
        colId: "hora_ingreso",
        headerName: "Hora",
        flex: 0.9,
        minWidth: 90,
        // `valueGetter` (no `field`+`valueFormatter`) a propósito: si lee
        // el string ISO crudo, AG Grid vuelve a inferirla como fecha (con
        // el mismo problema de la columna Fecha). Devolviendo ya el texto
        // "HH:MM", la columna se filtra/ordena como texto plano — mismo
        // comportamiento que el resto de columnas de texto, sin ícono raro.
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_ingreso) : ""),
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
        headerName: "Acción",
        flex: 0.9,
        minWidth: 90,
        filter: false,
        sortable: false,
        // Sin pinned a propósito: una columna fijada reserva el ancho del
        // scrollbar vertical aunque no haga falta scroll, dejando un
        // espacio en blanco justo antes de ella — con las 10 columnas ya
        // entrando sin scroll horizontal (ver el ajuste de anchos previo),
        // fijarla no aporta nada y sí ese espacio no deseado.
        cellRenderer: (p: ICellRendererParams<FilaActiva>) => {
          const fila = p.data;
          return fila ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
              onClick={() => salidaIndividual(fila)}
            >
              Salida
            </button>
          ) : null;
        },
      },
    ],
    [salidaIndividual],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<FilaActiva>
            cargando={cargando}
            filtrosPorColumna
            id="activos"
            idFila={claveFilaActiva}
            claseFila={claseFilaActiva}
            columnas={columnas}
            filas={filasVisibles}
            busqueda={busqueda}
            seleccionMultiple
            onSeleccionCambia={setSeleccionadas}
            accionesDerecha={<ToggleGafete filtro={filtroGafete} conteo={conteo} onCambiar={setFiltroGafete} />}
            controles={
              <>
                {/* Ingreso y salida como íconos, en una sola pieza (pedido
                    del usuario 2026-09-23); el atajo va en el nombre. */}
                <div className="segmentado" role="group" aria-label="Movimientos">
                  <button
                    type="button"
                    title="Nuevo ingreso (Ctrl+N)"
                    aria-label="Nuevo ingreso"
                    onClick={onAbrirNuevoIngreso}
                  >
                    <LogIn size={16} aria-hidden="true" />
                  </button>
                  <button
                    type="button"
                    title="Registrar salida (Ctrl+S)"
                    aria-label="Registrar salida"
                    onClick={onAbrirSalida}
                  >
                    <LogOut size={16} aria-hidden="true" />
                  </button>
                </div>
                <div className="campo" style={{ flex: "0 1 16rem" }}>
                  <input
                    placeholder="Cédula, nombre, empresa…"
                    value={busqueda}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
              </>
            }
          />
        </div>
      </div>

      {confirmarSalidaMasiva && (
        <Modal titulo="Confirmar salida" onCerrar={() => setConfirmarSalidaMasiva(false)}>
          <p style={{ marginTop: 0 }}>
            ¿Registrar la salida de {seleccionadas.length} persona(s)?
          </p>
          <ul style={{ margin: "0 0 1rem", paddingLeft: "1.2rem", color: "var(--muted)" }}>
            {seleccionadas.map((fila) => (
              <li key={claveFilaActiva(fila)}>
                {fila.contratista_nombre}
              </li>
            ))}
          </ul>
          <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
            <button
              type="button"
              className="boton"
              onClick={() => setConfirmarSalidaMasiva(false)}
              disabled={procesando}
            >
              Cancelar
            </button>
            <button
              type="button"
              className="boton boton-primario"
              onClick={confirmarSalidasSeleccionadas}
              disabled={procesando}
            >
              {procesando ? "Registrando…" : "Confirmar"}
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
