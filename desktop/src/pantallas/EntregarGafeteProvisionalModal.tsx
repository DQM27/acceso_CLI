import { useEffect, useMemo, useState } from "react";
import type { ChangeEvent } from "react";
import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import { listarEncargadosRuta } from "../api/rutas";
import type { EncargadoRuta } from "../api/rutas";
import { entregarGafeteProvisional } from "../api/gafetesProvisionales";

interface ValoresFormulario {
  gafete_numero: number;
}

const esquema = z.object({
  gafete_numero: z
    .number()
    .refine((n) => Number.isInteger(n) && n > 0, "El número de gafete es obligatorio"),
});

/** Mismo combobox nativo (`<input list>` + `<datalist>`) que
 * `IngresoProveedorModal` usa para empresa -- el catálogo de encargados ya
 * se carga completo en `PantallaRutas` (`listarEncargadosRuta`), así que
 * reusarlo acá evita un buscador servidor-lado aparte. Único campo
 * bloqueante (pedido explícito del usuario, mismo criterio que la versión
 * móvil): no tiene sentido prestar un gafete a texto libre sin encargado
 * real detrás. */
export default function EntregarGafeteProvisionalModal({
  onRegistrado,
  onCerrar,
}: {
  onRegistrado: () => void;
  onCerrar: () => void;
}) {
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ValoresFormulario>({
    resolver: zodResolver(esquema),
    // String vacío, no NaN -- con `type="number"` el DOM mostraba NaN en
    // blanco solo, pero con `type="text"` (ver abajo) NaN se refleja
    // literal como el string "NaN" en el campo (docs/pendientes.md,
    // "Auditar máscaras de entrada" -- mismo fix que IngresoProveedorModal).
    defaultValues: { gafete_numero: "" as unknown as number },
  });

  // `type="number"` deja teclear "e"/"-"/"+" (notación científica) aunque
  // el campo sea un entero positivo -- texto + filtrado en onChange, mismo
  // criterio que FormularioGafete.tsx/IngresoProveedorModal.tsx.
  const registroGafeteNumero = register("gafete_numero", {
    setValueAs: (valor: string) => (valor === "" ? Number.NaN : Number(valor)),
  });
  const alCambiarGafeteNumero = (evento: ChangeEvent<HTMLInputElement>) => {
    evento.target.value = evento.target.value.replace(/\D/g, "");
    registroGafeteNumero.onChange(evento);
  };

  const [encargados, setEncargados] = useState<EncargadoRuta[]>([]);
  useEffect(() => {
    // El núcleo filtra los desactivados (`soloActivos: true`) -- no queda
    // del lado de la pantalla decidir eso.
    listarEncargadosRuta(true)
      .then(setEncargados)
      .catch(() => {});
  }, []);

  const [encargadoTexto, setEncargadoTexto] = useState("");
  const [errorEncargado, setErrorEncargado] = useState<string | null>(null);

  const etiquetaEncargado = (encargado: EncargadoRuta) =>
    `${encargado.nombre} · ${encargado.codigo_empleado}`;

  const encargadoElegido = useMemo(() => {
    const texto = encargadoTexto.trim().toLowerCase();
    if (!texto) return null;
    return encargados.find((encargado) => etiquetaEncargado(encargado).toLowerCase() === texto) ?? null;
  }, [encargadoTexto, encargados]);

  async function alGuardar(valores: ValoresFormulario) {
    if (!encargadoElegido) {
      setErrorEncargado("Elija un encargado del catálogo");
      return;
    }
    setErrorEncargado(null);
    try {
      await entregarGafeteProvisional(encargadoElegido.id, valores.gafete_numero);
      onRegistrado();
    } catch (error) {
      setError("root", { message: String(error) });
    }
  }

  return (
    <Modal titulo="Entregar gafete provisional KOF" onCerrar={onCerrar}>
      <form
        onSubmit={handleSubmit(alGuardar)}
        style={{ display: "flex", flexDirection: "column", gap: "0.75rem", width: "24rem", maxWidth: "100%" }}
      >
        <label className="campo">
          Encargado
          <input
            list="encargados-provisional-datalist"
            value={encargadoTexto}
            onChange={(evento) => setEncargadoTexto(evento.target.value)}
            autoFocus
            autoComplete="off"
            placeholder="Escriba para buscar por nombre o código…"
          />
          <datalist id="encargados-provisional-datalist">
            {encargados.map((encargado) => (
              <option key={encargado.id} value={etiquetaEncargado(encargado)} />
            ))}
          </datalist>
        </label>
        {errorEncargado && <span style={{ color: "var(--error)" }}>{errorEncargado}</span>}
        {encargadoElegido && (
          <p style={{ color: "var(--muted)", fontSize: "0.85rem" }}>
            Código de empleado: {encargadoElegido.codigo_empleado} — corroborar contra lo que dice la
            persona
          </p>
        )}

        <label className="campo" style={{ flex: "0 1 10rem" }}>
          N.° de gafete provisional
          <input
            {...registroGafeteNumero}
            onChange={alCambiarGafeteNumero}
            inputMode="numeric"
          />
          {errors.gafete_numero && (
            <span style={{ color: "var(--error)" }}>{errors.gafete_numero.message}</span>
          )}
        </label>

        {errors.root && <p style={{ color: "var(--error)" }}>{errors.root.message}</p>}

        <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
          <button type="button" className="boton" onClick={onCerrar}>
            Cancelar
          </button>
          <button type="submit" className="boton boton-primario" disabled={isSubmitting}>
            {isSubmitting ? "Entregando…" : "Entregar"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
