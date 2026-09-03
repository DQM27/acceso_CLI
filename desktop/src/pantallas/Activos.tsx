import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import PantallaEncabezado from "../componentes/PantallaEncabezado";
import {
  cerrarIngresoRemoto,
  listarIngresosActivos,
  listarIngresosRemotos,
  mensajeMotivoDenegacion,
  registrarSalida,
  textoMedio,
} from "../api";
import type { IngresoActivoResumen, IngresoRemoto } from "../api";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

/** Texto plano del estado — separado del componente visual `EstadoAcceso`
 * para que la columna tenga un `field` real: sin eso no aparece en el
 * selector "Columnas ▾" (que sólo lista columnas con `field`) ni el filtro
 * por columna puede buscar sobre ella. */
export function textoEstado(fila: IngresoActivoResumen): string {
  const r = fila.resultado_acceso;
  if (r === "Permitido") return "Al día";
  if (r === "PermitidoConAdvertencia") return "PRAIND próximo a vencer";
  return mensajeMotivoDenegacion(r.Denegado);
}

export function colorEstado(fila: IngresoActivoResumen): string {
  const r = fila.resultado_acceso;
  if (r === "Permitido") return "var(--exito)";
  if (r === "PermitidoConAdvertencia") return "var(--advertencia)";
  return "var(--error)";
}

/** Fila local (este dispositivo) o remota (abierta por el otro dispositivo
 * del mismo sitio, ver `docs/plan-persistencia-nube.md` — nunca vive en el
 * historial local, sólo en la caché `ingresos_remotos`). Mismos nombres de
 * campo en los dos casos (los que una remota no tiene van en `null`) para
 * que las columnas de AG Grid no necesiten saber cuál es cuál salvo donde
 * de verdad importa (estado, acción). */
interface FilaLocal extends IngresoActivoResumen {
  origen: "local";
  estado_texto: string;
}

interface FilaRemota {
  origen: "remoto";
  uuid_remoto: string;
  registro_id: null;
  contratista_id: null;
  cedula: null;
  contratista_nombre: string;
  empresa_nombre: null;
  tipo_ingreso: null;
  medio_ingreso: null;
  fecha_hora_ingreso: string;
  gafete_numero: null;
  usuario_ingreso_nombre: string;
  resultado_registrado: null;
  resultado_acceso: null;
  estado_texto: string;
}

type FilaActiva = FilaLocal | FilaRemota;

export function filaDesdeLocal(item: IngresoActivoResumen): FilaActiva {
  return { ...item, origen: "local", estado_texto: textoEstado(item) };
}

export function filaDesdeRemoto(remoto: IngresoRemoto): FilaActiva {
  return {
    origen: "remoto",
    uuid_remoto: remoto.uuid,
    registro_id: null,
    contratista_id: null,
    cedula: null,
    contratista_nombre: remoto.contratista_nombre,
    empresa_nombre: null,
    tipo_ingreso: null,
    medio_ingreso: null,
    fecha_hora_ingreso: remoto.hora_entrada,
    gafete_numero: null,
    usuario_ingreso_nombre: remoto.usuario_entrada_nombre ?? "—",
    resultado_registrado: null,
    resultado_acceso: null,
    estado_texto: "Otro dispositivo",
  };
}

function textoEstadoFila(fila: FilaActiva): string {
  return fila.origen === "remoto" ? fila.estado_texto : textoEstado(fila);
}

function colorEstadoFila(fila: FilaActiva): string {
  return fila.origen === "remoto" ? "var(--acento)" : colorEstado(fila);
}

function EstadoAcceso({ fila }: { fila: FilaActiva }) {
  return (
    <span className="chip" style={{ ["--chip-color" as string]: colorEstadoFila(fila) }}>
      {textoEstadoFila(fila)}
    </span>
  );
}

export default function Activos({
  refrescarSenal,
  onAbrirNuevoIngreso,
  onAbrirSalida,
}: {
  /** Los modales de Nuevo Ingreso y Salida viven en el Shell (se disparan
   * desde cualquier pantalla vía Ctrl+Shift+N/S, no sólo desde acá) —
   * este número sube cada vez que registran algo, para que la grilla se
   * refresque aunque ya estuviera montada. */
  refrescarSenal?: number;
  onAbrirNuevoIngreso: () => void;
  onAbrirSalida: () => void;
}) {
  const [filas, setFilas] = useState<FilaActiva[]>([]);
  const [total, setTotal] = useState(0);
  const [cargando, setCargando] = useState(true);
  const [busqueda, setBusqueda] = useState("");
  const [seleccionadas, setSeleccionadas] = useState<FilaActiva[]>([]);
  const [confirmarSalidaMasiva, setConfirmarSalidaMasiva] = useState(false);
  const [procesando, setProcesando] = useState(false);

  const recargar = useCallback(() => {
    setCargando(true);
    // `listarIngresosRemotos` no hace red -- lee la caché local que ya
    // llenó la última sincronización (manual o automática); si la nube
    // nunca se configuró en este dispositivo, simplemente devuelve una
    // lista vacía, no falla.
    return Promise.all([listarIngresosActivos(), listarIngresosRemotos()])
      .then(([pagina, remotos]) => {
        setFilas([...pagina.items.map(filaDesdeLocal), ...remotos.map(filaDesdeRemoto)]);
        setTotal(pagina.total + remotos.length);
        setSeleccionadas([]);
      })
      .finally(() => setCargando(false));
  }, []);

  useEffect(() => {
    let vigente = true;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
  }, [recargar, refrescarSenal]);

  /** Local: cierra en `registro_ingresos` (este dispositivo). Remota: cierra
   * directo contra la nube (`nube::cerrar_ingreso_remoto`) -- nunca toca el
   * historial local, esa fila no es -- ni fue -- de este dispositivo. */
  async function cerrarFila(fila: FilaActiva) {
    if (fila.origen === "local") {
      await registrarSalida(fila.registro_id);
    } else {
      await cerrarIngresoRemoto(fila.uuid_remoto);
    }
  }

  // useCallback a propósito: esta función se cierra dentro de una celda de
  // `columnas` — sin identidad estable, `columnas` (memoizado más abajo)
  // se recrearía en cada render igual, y con eso AG Grid reasignaría el
  // orden/ancho originales del código encima de lo que el usuario acomodó
  // (ver el comentario junto a `columnas`).
  const salidaIndividual = useCallback(
    async (fila: FilaActiva) => {
      try {
        await cerrarFila(fila);
        recargar();
      } catch (error) {
        toast.error(String(error));
      }
    },
    [recargar],
  );

  async function confirmarSalidasSeleccionadas() {
    setProcesando(true);
    try {
      for (const fila of seleccionadas) {
        await cerrarFila(fila);
      }
      setConfirmarSalidaMasiva(false);
      await recargar();
    } catch (error) {
      toast.error(String(error));
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
      { field: "tipo_ingreso", headerName: "Tipo", flex: 1, minWidth: 100 },
      {
        field: "medio_ingreso",
        headerName: "Medio",
        flex: 1,
        minWidth: 100,
        valueFormatter: (p) => (p.value == null ? "—" : textoMedio(p.value)),
      },
      {
        field: "gafete_numero",
        headerName: "Gafete",
        flex: 0.9,
        minWidth: 90,
        valueFormatter: (p) =>
          p.data?.origen === "remoto" ? "—" : p.value == null ? "S/G" : String(p.value),
      },
      {
        colId: "fecha_ingreso",
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
      },
      { field: "usuario_ingreso_nombre", headerName: "Dio ingreso", flex: 1.3, minWidth: 130 },
      {
        field: "estado_texto",
        headerName: "Estado",
        flex: 1.7,
        minWidth: 170,
        cellRenderer: (p: ICellRendererParams<FilaActiva>) =>
          p.data ? <EstadoAcceso fila={p.data} /> : null,
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
        cellRenderer: (p: ICellRendererParams<FilaActiva>) =>
          p.data ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
              onClick={() => salidaIndividual(p.data!)}
            >
              Salida
            </button>
          ) : null,
      },
    ],
    [salidaIndividual],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <PantallaEncabezado titulo="Ingresos activos" />

      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        {seleccionadas.length > 0 && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              padding: "0.5rem 0.75rem",
              border: "1px solid var(--borde)",
              borderRadius: "var(--radio-chico)",
              background: "var(--campo-fondo)",
            }}
          >
            <span style={{ color: "var(--texto)", fontSize: "0.9rem" }}>
              {seleccionadas.length} seleccionado(s)
            </span>
            <button
              type="button"
              className="boton boton-primario"
              onClick={() => setConfirmarSalidaMasiva(true)}
            >
              Registrar salida ({seleccionadas.length})
            </button>
          </div>
        )}

        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<FilaActiva>
            id="activos"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            seleccionMultiple
            onSeleccionCambia={setSeleccionadas}
            controles={
              <>
                <button className="boton" title="Ctrl+Shift+N" onClick={onAbrirNuevoIngreso}>
                  + Nuevo
                </button>
                <div className="campo" style={{ flex: "1 1 16rem" }}>
                  <input
                    placeholder="Cédula, nombre, empresa…"
                    value={busqueda}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
              </>
            }
            accionesDerecha={
              <button className="boton" title="Ctrl+Shift+S" onClick={onAbrirSalida}>
                Salida
              </button>
            }
          />
        </div>
        <p style={{ color: "var(--muted)", margin: 0 }}>
          {cargando ? "Cargando…" : `${total} adentro`}
        </p>
      </div>

      {confirmarSalidaMasiva && (
        <Modal titulo="Confirmar salida" onCerrar={() => setConfirmarSalidaMasiva(false)}>
          <p style={{ marginTop: 0 }}>
            ¿Registrar la salida de {seleccionadas.length} persona(s)?
          </p>
          <ul style={{ margin: "0 0 1rem", paddingLeft: "1.2rem", color: "var(--muted)" }}>
            {seleccionadas.map((fila) => (
              <li key={fila.origen === "local" ? `local-${fila.registro_id}` : `remoto-${fila.uuid_remoto}`}>
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
