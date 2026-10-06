import { useEffect, useState } from "react";
import { listarTodosLosCorreosActivos, registrarIngresoCorreo } from "../api/correo";
import type { MedioIngreso } from "../api";
import { textoGafete, validarNumeroGafete } from "../busqueda";
import { nombreMientrasSeEscribe } from "../nombres";

/**
 * Visita autorizada por correo: el guarda la registra a mano porque quien
 * la autoriza mandó un correo en vez de agendarla en la web. Lo usa
 * `NuevaVisitaModal` en el modo "Por correo" (antes era la sección aparte
 * "Por correo"). Gafete de VISITA obligatorio. Medio de ingreso igual que el
 * de contratistas: "Caminando" o "Vehículo" con su placa.
 * Las reglas las valida el núcleo (`registrar_ingreso_correo_verificado`);
 * acá sólo se pide lo mínimo para no mandar un formulario incompleto. No se
 * cierra al registrar, igual que el check-in: queda listo para la próxima.
 */
export function FormularioPorCorreo({
  cedulaInicial = "",
  onRegistrado,
}: {
  cedulaInicial?: string;
  onRegistrado: () => void;
}) {
  const [gafetesAdentro, setGafetesAdentro] = useState<Map<string, number>>(new Map());
  const [cedula, setCedula] = useState(cedulaInicial);
  const [nombre, setNombre] = useState("");
  const [motivo, setMotivo] = useState("");
  const [medio, setMedio] = useState<MedioIngreso>("Caminando");
  const [placa, setPlaca] = useState("");
  const [gafeteTexto, setGafeteTexto] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [mensaje, setMensaje] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  useEffect(() => {
    // Sólo alimenta el aviso "ya está adentro": si falla, igual se registra
    // (el núcleo vuelve a verificar).
    listarTodosLosCorreosActivos()
      .then((activos) =>
        setGafetesAdentro(new Map(activos.map((fila) => [fila.cedula.trim(), fila.gafete_numero]))),
      )
      .catch(() => {});
  }, [mensaje]);

  async function registrar() {
    if (!cedula.trim()) return setError("La cédula es obligatoria");
    if (!nombre.trim()) return setError("El nombre es obligatorio");
    if (!motivo.trim()) return setError("Indique a quién visita y quién lo autorizó por correo");
    if (medio === "Vehiculo" && !placa.trim()) return setError("Escriba la placa del vehículo");
    const gafete = validarNumeroGafete(gafeteTexto);
    if (!gafete.valido) return setError(gafete.mensaje);
    setError(null);
    setEnviando(true);
    try {
      await registrarIngresoCorreo({
        cedula: cedula.trim(),
        nombre: nombre.trim(),
        motivo: motivo.trim(),
        // Sin placa = caminando (así lo guarda la nube).
        placa: medio === "Vehiculo" ? placa.trim() : null,
        gafete_numero: gafete.numero,
      });
      setMensaje(`✓ Entrada registrada — ${nombre.trim()}`);
      setCedula("");
      setNombre("");
      setMotivo("");
      setMedio("Caminando");
      setPlaca("");
      setGafeteTexto("");
      onRegistrado();
    } catch (error) {
      setError(String(error));
    } finally {
      setEnviando(false);
    }
  }

  const gafeteAdentro = gafetesAdentro.get(cedula.trim());

  return (
    <form
      onSubmit={(evento) => {
        evento.preventDefault();
        registrar();
      }}
      style={{ display: "flex", flexDirection: "column", gap: "0.9rem" }}
    >
      <p style={{ margin: 0, color: "var(--muted)", fontSize: "0.85rem" }}>
        Sólo con un correo que autorice la visita. Si la visita está agendada en la web, use “Agendada”.
      </p>

      <div style={{ display: "flex", gap: "0.75rem" }}>
        <label className="campo" style={{ flex: 1 }}>
          Cédula
          <input
            value={cedula}
            onChange={(evento) => {
              setCedula(evento.target.value);
              setMensaje(null);
            }}
            autoFocus={!cedulaInicial}
            autoComplete="off"
          />
        </label>
        <label className="campo" style={{ flex: 1.6 }}>
          Nombre
          <input
            value={nombre}
            onChange={(evento) => setNombre(nombreMientrasSeEscribe(evento.target.value))}
            autoFocus={!!cedulaInicial}
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
        A quién visita y quién lo autorizó
        <input
          value={motivo}
          onChange={(evento) => setMotivo(evento.target.value)}
          autoComplete="off"
          placeholder="Ej.: Entrevista RH – autorizó Ana Mora por correo"
        />
      </label>

      <div className="campo">
        Medio de ingreso
        <div style={{ display: "flex", gap: "0.75rem" }}>
          {(["Caminando", "Vehiculo"] as const).map((opcion) => (
            <label key={opcion} style={{ display: "flex", alignItems: "center", gap: "0.4rem", color: "var(--texto)" }}>
              <input
                type="radio"
                name="medio-correo"
                checked={medio === opcion}
                onChange={() => {
                  setMedio(opcion);
                  if (opcion === "Caminando") setPlaca("");
                }}
              />
              {opcion === "Caminando" ? "Caminando" : "Vehículo"}
            </label>
          ))}
        </div>
      </div>

      <div style={{ display: "flex", gap: "0.75rem" }}>
        {medio === "Vehiculo" && (
          <label className="campo" style={{ flex: 1 }}>
            Placa del vehículo
            <input
              value={placa}
              onChange={(evento) => setPlaca(evento.target.value.toUpperCase())}
              autoFocus
              autoComplete="off"
              placeholder="Placa del vehículo"
            />
          </label>
        )}
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

      {mensaje && <p style={{ color: "var(--exito)", margin: 0 }}>{mensaje}</p>}

      {error && (
        <p className="login-error" role="alert">
          {error}
        </p>
      )}

      <div style={{ display: "flex", justifyContent: "flex-end" }}>
        <button type="submit" className="boton boton-primario" disabled={enviando}>
          {enviando ? "Registrando…" : "Registrar entrada"}
        </button>
      </div>
    </form>
  );
}
