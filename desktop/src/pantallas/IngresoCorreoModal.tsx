import { useEffect, useState } from "react";
import Modal from "../componentes/Modal";
import { listarTodosLosCorreosActivos, registrarIngresoCorreo } from "../api/correo";
import { textoGafete, validarNumeroGafete } from "../busqueda";

/**
 * Registro de un ingreso "por correo" (visita autorizada por correo,
 * generalmente entrevistas de RH): comodín mientras se termina el módulo de
 * Visitas. Mismo formulario que el de proveedores, sin buscador de
 * conocidos (casi nunca se repiten) y con el motivo ("a quién visita") en
 * vez de la empresa. El gafete es de VISITA y obligatorio; la placa vacía
 * significa que llegó caminando. Las reglas las valida el núcleo
 * (`registrar_ingreso_correo_verificado`); acá sólo se pide lo mínimo para
 * no mandar un formulario incompleto.
 */
export default function IngresoCorreoModal({
  onRegistrado,
  onCerrar,
}: {
  onRegistrado: () => void;
  onCerrar: () => void;
}) {
  const [gafetesAdentro, setGafetesAdentro] = useState<Map<string, number>>(new Map());
  const [cedula, setCedula] = useState("");
  const [nombre, setNombre] = useState("");
  const [motivo, setMotivo] = useState("");
  const [placa, setPlaca] = useState("");
  const [gafeteTexto, setGafeteTexto] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  useEffect(() => {
    // Sólo alimenta el aviso "ya está adentro": si falla, igual se registra
    // (el núcleo vuelve a verificar).
    listarTodosLosCorreosActivos()
      .then((activos) =>
        setGafetesAdentro(new Map(activos.map((fila) => [fila.cedula.trim(), fila.gafete_numero]))),
      )
      .catch(() => {});
  }, []);

  async function registrar() {
    if (!cedula.trim()) return setError("La cédula es obligatoria");
    if (!nombre.trim()) return setError("El nombre es obligatorio");
    if (!motivo.trim()) return setError("Indique el motivo de la visita");
    const gafete = validarNumeroGafete(gafeteTexto);
    if (!gafete.valido) return setError(gafete.mensaje);
    setError(null);
    setEnviando(true);
    try {
      await registrarIngresoCorreo({
        cedula: cedula.trim(),
        nombre: nombre.trim(),
        motivo: motivo.trim(),
        placa: placa.trim() || null,
        gafete_numero: gafete.numero,
      });
      onRegistrado();
    } catch (error) {
      setError(String(error));
      setEnviando(false);
    }
  }

  const gafeteAdentro = gafetesAdentro.get(cedula.trim());

  return (
    <Modal titulo="Nuevo ingreso por correo" onCerrar={onCerrar}>
      <form
        onSubmit={(evento) => {
          evento.preventDefault();
          registrar();
        }}
        style={{ display: "flex", flexDirection: "column", gap: "0.9rem" }}
      >
        <div style={{ display: "flex", gap: "0.75rem" }}>
          <label className="campo" style={{ flex: 1 }}>
            Cédula
            <input
              value={cedula}
              onChange={(evento) => setCedula(evento.target.value)}
              autoFocus
              autoComplete="off"
            />
          </label>
          <label className="campo" style={{ flex: 1.6 }}>
            Nombre
            <input
              value={nombre}
              onChange={(evento) => setNombre(evento.target.value.toUpperCase())}
              autoComplete="off"
            />
          </label>
        </div>

        {gafeteAdentro !== undefined && (
          <p style={{ margin: 0, color: "var(--advertencia)", fontSize: "0.85rem" }}>
            ⚠ Ya está adentro con el gafete {textoGafete(gafeteAdentro)}
          </p>
        )}

        <label className="campo">
          Motivo de la visita
          <input
            value={motivo}
            onChange={(evento) => setMotivo(evento.target.value)}
            autoComplete="off"
            placeholder="Ej.: Entrevista RH – a quién visita"
          />
        </label>

        <div style={{ display: "flex", gap: "0.75rem" }}>
          <label className="campo" style={{ flex: 1 }}>
            Placa (opcional)
            <input
              value={placa}
              onChange={(evento) => setPlaca(evento.target.value.toUpperCase())}
              autoComplete="off"
              placeholder="Vacía = caminando"
            />
          </label>
          <label className="campo" style={{ flex: 1 }}>
            N.° de gafete de visita
            <input
              value={gafeteTexto}
              onChange={(evento) => setGafeteTexto(evento.target.value.replace(/\D/g, ""))}
              inputMode="numeric"
              autoComplete="off"
              placeholder="Número de gafete"
            />
          </label>
        </div>

        {error && (
          <p className="login-error" role="alert">
            {error}
          </p>
        )}

        <div style={{ display: "flex", justifyContent: "flex-end" }}>
          <button type="submit" className="boton boton-primario" disabled={enviando}>
            {enviando ? "Registrando…" : "Registrar ingreso"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
