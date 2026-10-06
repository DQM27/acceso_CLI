import { useCallback, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import InterruptorCelda from "../componentes/InterruptorCelda";
import { useLista } from "../componentes/useLista";
import { cambiarEstadoAnfitrion, crearAnfitrion, listarAnfitriones, restablecerAnfitrion } from "../api/anfitriones";
import type { Anfitrion, CodigoEmitido, EstadoAnfitrion } from "../api/anfitriones";
import { reglas } from "../reglas";
import { textoFechaDDMMYYYY, fechaLocalYMD, textoHora } from "../tiempo";
import { mensajeError } from "../mensajeError";

/**
 * Cuentas de anfitriones de la web de visitas (ver `api/anfitriones.ts`).
 * Mismo modelo que los usuarios de la portería: el alta y "Restablecer"
 * muestran UNA vez un código de activación para entregarle a la persona; con
 * él elige su contraseña en la web de visitas («Primer ingreso»). Sin correos
 * ni SMTP. El interruptor "Activo" deshabilita o habilita la cuenta (bloquea
 * el ingreso y cierra sus sesiones).
 */

const ESTADOS: Record<EstadoAnfitrion, { texto: string; color: string }> = {
  activa: { texto: "Activa", color: "var(--exito)" },
  pendiente: { texto: "Pendiente de activar", color: "var(--info)" },
  codigo_vencido: { texto: "Código vencido", color: "var(--advertencia)" },
  sin_contrasena: { texto: "Sin contraseña", color: "var(--advertencia)" },
  sin_cuenta: { texto: "Sin cuenta", color: "var(--advertencia)" },
  deshabilitada: { texto: "Deshabilitada", color: "var(--muted)" },
};

function fechaHora(iso: string): string {
  return `${textoFechaDDMMYYYY(fechaLocalYMD(iso))} ${textoHora(iso)}`;
}

/** "ABCDE23456" → "ABCDE-23456": más fácil de dictar; la web acepta las dos. */
function codigoLegible(codigo: string): string {
  return codigo.length === 10 ? `${codigo.slice(0, 5)}-${codigo.slice(5)}` : codigo;
}

export default function Anfitriones() {
  const [busqueda, setBusqueda] = useState("");
  const [modalAbierto, setModalAbierto] = useState(false);
  const [correo, setCorreo] = useState("");
  const [nombre, setNombre] = useState("");
  const [creando, setCreando] = useState(false);
  const [errorForm, setErrorForm] = useState<string | null>(null);
  // Se muestra una sola vez, apenas vuelve de la Edge Function.
  const [codigoEmitido, setCodigoEmitido] = useState<CodigoEmitido | null>(null);
  const [restableciendo, setRestableciendo] = useState<string | null>(null);

  // Sin aviso en vivo: `anfitriones` cambia solo desde esta pantalla.
  const { datos, cargando, recargar } = useLista(["anfitriones"], listarAnfitriones, {
    intervaloMs: 120_000,
    tablas: "",
  });
  const filas = useMemo(() => datos ?? [], [datos]);

  function cerrarModal() {
    setModalAbierto(false);
    setCorreo("");
    setNombre("");
    setErrorForm(null);
  }

  async function alEnviarFormulario(evento: React.FormEvent) {
    evento.preventDefault();
    setCreando(true);
    setErrorForm(null);
    try {
      const emitido = await crearAnfitrion({ correo: correo.trim().toLowerCase(), nombre: nombre.trim() });
      cerrarModal();
      void recargar();
      setCodigoEmitido(emitido);
    } catch (error) {
      setErrorForm(mensajeError(error));
    } finally {
      setCreando(false);
    }
  }

  const manejarRestablecer = useCallback(
    async (fila: Anfitrion) => {
      setRestableciendo(fila.correo);
      try {
        setCodigoEmitido(await restablecerAnfitrion(fila.correo));
        void recargar();
      } catch (error) {
        toast.error(mensajeError(error));
      } finally {
        setRestableciendo(null);
      }
    },
    [recargar],
  );

  async function manejarEdicion(fila: Anfitrion) {
    try {
      await cambiarEstadoAnfitrion(fila.correo, fila.activo);
      toast.success(fila.activo ? `${fila.nombre}: cuenta habilitada.` : `${fila.nombre}: cuenta deshabilitada.`);
    } catch (error) {
      toast.error(mensajeError(error));
    } finally {
      // También en el éxito: el estado ("Deshabilitada", "Activa") lo calcula el servidor.
      void recargar();
    }
  }

  const columnas: ColDef<Anfitrion>[] = useMemo(
    () => [
      { field: "nombre", headerName: "Nombre", flex: 1.4, minWidth: 170, cellStyle: { textAlign: "left" } },
      { field: "correo", headerName: "Correo", flex: 1.6, minWidth: 200, cellStyle: { textAlign: "left" } },
      {
        field: "estado",
        headerName: "Estado",
        flex: 1,
        minWidth: 160,
        valueFormatter: ({ value }) => ESTADOS[value as EstadoAnfitrion]?.texto ?? String(value),
        cellRenderer: ({ data }: { data: Anfitrion }) => {
          const { texto, color } = ESTADOS[data.estado];
          return (
            <span className="chip" style={{ ["--chip-color" as string]: color }}>
              {texto}
            </span>
          );
        },
      },
      {
        field: "codigo_vence",
        headerName: "Código vence",
        flex: 1,
        minWidth: 150,
        filter: false,
        valueFormatter: ({ value }) => (value ? fechaHora(value as string) : "—"),
      },
      {
        field: "activo",
        headerName: "Activo",
        flex: 0.8,
        minWidth: 100,
        cellRenderer: InterruptorCelda,
        cellRendererParams: { critico: true },
        filter: false,
      },
      {
        colId: "restablecer",
        headerName: "",
        flex: 1,
        minWidth: 150,
        filter: false,
        sortable: false,
        cellRenderer: ({ data }: { data: Anfitrion }) => (
          <button
            type="button"
            className="boton boton-celda-angosto"
            disabled={!data.activo || restableciendo === data.correo}
            title={data.activo ? undefined : "Habilite la cuenta primero"}
            onClick={() => manejarRestablecer(data)}
          >
            {restableciendo === data.correo
              ? "Generando…"
              : data.estado === "activa"
                ? "Restablecer contraseña"
                : "Generar código"}
          </button>
        ),
      },
    ],
    [restableciendo, manejarRestablecer],
  );

  return (
    <div className="flex h-full flex-col">
      <div className="pantalla-cuerpo min-h-0 flex-1">
        <div className="min-h-0 flex-1">
          <Tabla<Anfitrion>
            id="anfitriones"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            filtrosPorColumna
            onCeldaEditada={manejarEdicion}
            controles={
              <>
                <button type="button" className="boton" onClick={() => setModalAbierto(true)}>
                  + Nuevo
                </button>
                <div className="campo flex-[0_1_16rem]">
                  <input
                    placeholder="Nombre o correo…"
                    value={busqueda}
                    disabled={cargando}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
              </>
            }
          />
        </div>
      </div>

      {modalAbierto && (
        <Modal titulo="Nuevo anfitrión" onCerrar={cerrarModal}>
          <form onSubmit={alEnviarFormulario} className="flex flex-col gap-3">
            <label className="campo">
              Correo de la empresa
              <input
                required
                autoFocus
                type="email"
                autoCapitalize="none"
                spellCheck={false}
                maxLength={254}
                value={correo}
                disabled={creando}
                onChange={(evento) => setCorreo(evento.target.value)}
              />
            </label>

            <label className="campo">
              Nombre
              <input
                required
                maxLength={120}
                value={nombre}
                disabled={creando}
                onChange={(evento) => setNombre(reglas.nombreMientrasSeEscribe(evento.target.value))}
              />
            </label>

            <p className="m-0 text-[0.8rem] text-muted">
              Se genera un código de activación que se muestra una sola vez. Entrégueselo a la persona: con él elige
              su contraseña en la web de visitas. Vence en 72 horas.
            </p>

            {errorForm && (
              <p className="login-error" role="alert">
                {errorForm}
              </p>
            )}

            <div className="flex justify-end gap-2">
              <button type="button" className="boton" disabled={creando} onClick={cerrarModal}>
                Cancelar
              </button>
              <button type="submit" className="boton boton-primario" disabled={creando}>
                {creando ? "Creando…" : "Crear anfitrión"}
              </button>
            </div>
          </form>
        </Modal>
      )}

      {codigoEmitido && (
        <Modal titulo="Código de activación" onCerrar={() => setCodigoEmitido(null)}>
          <div className="flex flex-col gap-3">
            <p className="m-0 text-[0.85rem] text-muted">
              Para <strong>{codigoEmitido.nombre}</strong> ({codigoEmitido.correo}). Entrégueselo en persona o por un
              canal de confianza. No se vuelve a mostrar después de cerrar esta ventana.
            </p>
            <div className="flex items-center gap-2 rounded-[0.4rem] border border-borde px-[0.8rem] py-[0.6rem] font-mono text-[1.1rem] tracking-wider">
              <span className="flex-1">{codigoLegible(codigoEmitido.codigo)}</span>
              <button
                type="button"
                className="boton"
                onClick={() => {
                  void navigator.clipboard.writeText(codigoLegible(codigoEmitido.codigo));
                  toast.success("Copiado.");
                }}
              >
                Copiar
              </button>
            </div>
            <p className="m-0 text-[0.8rem] text-muted">
              Vence el {fechaHora(codigoEmitido.vence)} y sirve una sola vez. La persona entra a la web de visitas,
              elige «Primer ingreso: tengo un código de activación» y escribe su correo, este código y su contraseña
              nueva.
            </p>
            <div className="flex justify-end">
              <button type="button" className="boton boton-primario" onClick={() => setCodigoEmitido(null)}>
                Ya lo copié
              </button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
