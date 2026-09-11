import { useEffect, useRef, useState } from "react";
import Modal from "../componentes/Modal";
import { registrarEntradaVisita, verificarCheckInVisita } from "../api";
import type { MovimientoVisitaActivoResumen, PreparacionVisita } from "../api";

type Estado =
  | { tipo: "buscando" }
  | { tipo: "verificando" }
  | { tipo: "encontrada"; preparacion: PreparacionVisita }
  | { tipo: "ya-adentro"; nombre: string }
  | { tipo: "sin-cita"; mensaje: string };

/** El gafete es opcional acá (a diferencia de `NuevoIngresoModal`, donde
 * puede ser requerido) -- vacío siempre es válido con `numero: null`. */
export function validarGafeteOpcional(
  texto: string,
): { valido: true; numero: number | null } | { valido: false; mensaje: string } {
  const recortado = texto.trim();
  if (!recortado) return { valido: true, numero: null };
  const numero = Number.parseInt(recortado, 10);
  if (Number.isNaN(numero)) return { valido: false, mensaje: "Ingrese un número de gafete válido" };
  return { valido: true, numero };
}

/** `verificar_check_in_visita` sólo confirma que la cita es válida hoy, no
 * si el visitante ya entró (ver el doc-comment de `visitasActivas` más
 * abajo) -- este es el filtro de UI que sí lo revisa. */
export function estaYaAdentro(
  visitasActivas: MovimientoVisitaActivoResumen[],
  cedula: string,
): boolean {
  return visitasActivas.some((fila) => fila.cedula === cedula);
}

/**
 * Check-in de visitas por cédula -- mismo armazón que `NuevoIngresoModal`
 * (buscador arriba, panel de confirmación que se expande debajo, no se
 * cierra solo al registrar), pero sin buscador con lista flotante: la
 * cédula es exacta, no hay nada que buscar por texto parcial.
 */
export default function VisitaCheckInModal({
  visitasActivas,
  onRegistrado,
  onCerrar,
}: {
  /** Para bloquear "ya está adentro" desde el paso de Verificar, antes de
   * expandir el panel de confirmación -- `verificar_check_in_visita` sólo
   * confirma que la cita es válida hoy, no si el visitante ya entró; eso lo
   * decide recién `registrar_entrada` en el backend (la fuente de verdad
   * real, esto es sólo un filtro de UI para no hacer completar un
   * formulario que de todos modos va a fallar al confirmar). */
  visitasActivas: MovimientoVisitaActivoResumen[];
  onRegistrado: () => void;
  onCerrar: () => void;
}) {
  const [cedula, setCedula] = useState("");
  const [estado, setEstado] = useState<Estado>({ tipo: "buscando" });
  const [gafeteTexto, setGafeteTexto] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [mensaje, setMensaje] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);
  const cedulaRef = useRef<HTMLInputElement>(null);
  const confirmarRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (estado.tipo === "encontrada") {
      confirmarRef.current?.focus();
    }
  }, [estado]);

  function cambiarCedula(texto: string) {
    setCedula(texto);
    setMensaje(null);
    if (estado.tipo !== "buscando") {
      setEstado({ tipo: "buscando" });
    }
  }

  async function verificar() {
    const valor = cedula.trim();
    if (!valor) return;
    setError(null);
    setMensaje(null);
    setGafeteTexto("");
    setEstado({ tipo: "verificando" });
    try {
      const preparacion = await verificarCheckInVisita(valor);
      if (estaYaAdentro(visitasActivas, preparacion.visitante.cedula)) {
        setEstado({ tipo: "ya-adentro", nombre: preparacion.visitante.nombre });
        return;
      }
      setEstado({ tipo: "encontrada", preparacion });
    } catch (error) {
      setEstado({ tipo: "sin-cita", mensaje: String(error) });
    }
  }

  async function confirmarEntrada() {
    if (estado.tipo !== "encontrada") return;
    const resultado = validarGafeteOpcional(gafeteTexto);
    if (!resultado.valido) {
      setError(resultado.mensaje);
      return;
    }
    setError(null);
    setEnviando(true);
    try {
      await registrarEntradaVisita(estado.preparacion.visitante.cedula, resultado.numero);
      setMensaje(`✓ Entrada registrada — ${estado.preparacion.visitante.nombre}`);
      setEstado({ tipo: "buscando" });
      setCedula("");
      cedulaRef.current?.focus();
      onRegistrado();
    } catch (error) {
      setError(String(error));
    } finally {
      setEnviando(false);
    }
  }

  return (
    <Modal titulo="Nueva visita" onCerrar={onCerrar}>
      <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
        <form
          onSubmit={(evento) => {
            evento.preventDefault();
            verificar();
          }}
          style={{ display: "flex", gap: "0.75rem", alignItems: "flex-end" }}
        >
          <label className="campo" style={{ flex: 1 }}>
            Cédula del visitante
            <input
              ref={cedulaRef}
              value={cedula}
              onChange={(evento) => cambiarCedula(evento.target.value)}
              autoFocus
              placeholder="Escanee o escriba la cédula…"
            />
          </label>
          <button
            type="submit"
            className="boton boton-primario"
            disabled={estado.tipo === "verificando" || !cedula.trim()}
          >
            {estado.tipo === "verificando" ? "Verificando…" : "Verificar"}
          </button>
        </form>

        {mensaje && <p style={{ color: "var(--exito)", margin: 0 }}>{mensaje}</p>}

        {estado.tipo === "sin-cita" && (
          <p className="login-error" role="alert">
            {estado.mensaje}
          </p>
        )}

        {estado.tipo === "ya-adentro" && (
          <p className="login-error" role="alert">
            {estado.nombre} ya tiene un ingreso activo — registre la salida antes de volver a entrar.
          </p>
        )}

        {estado.tipo === "encontrada" && (
          <form
            onSubmit={(evento) => {
              evento.preventDefault();
              confirmarEntrada();
            }}
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "0.9rem",
              padding: "0.85rem",
              border: "1px solid var(--borde)",
              borderRadius: "var(--radio-chico)",
              background: "var(--campo-fondo)",
            }}
          >
            <div>
              <p style={{ margin: 0, fontWeight: 600, color: "var(--texto)" }}>
                {estado.preparacion.visitante.nombre}
              </p>
              <p style={{ margin: "0.15rem 0 0", color: "var(--muted)", fontSize: "0.85rem" }}>
                {estado.preparacion.visitante.cedula}
                {estado.preparacion.visitante.empresa && ` · ${estado.preparacion.visitante.empresa}`}
                {" · Anfitrión: "}
                {estado.preparacion.cita.anfitrion_nombre}
                {estado.preparacion.cita.motivo && ` · ${estado.preparacion.cita.motivo}`}
                {estado.preparacion.cita.hora_estimada &&
                  ` · Llegada estimada: ${estado.preparacion.cita.hora_estimada.slice(0, 5)}`}
              </p>
            </div>

            <label className="campo">
              Número de gafete (opcional)
              <input
                value={gafeteTexto}
                onChange={(evento) => setGafeteTexto(evento.target.value.replace(/\D/g, ""))}
                inputMode="numeric"
                autoFocus
                placeholder="S/G si vacío"
              />
            </label>

            {error && (
              <p className="login-error" role="alert">
                {error}
              </p>
            )}

            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <button ref={confirmarRef} type="submit" className="boton boton-primario" disabled={enviando}>
                {enviando ? "Registrando…" : "Registrar entrada"}
              </button>
            </div>
          </form>
        )}
      </div>
    </Modal>
  );
}
