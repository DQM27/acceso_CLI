import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import { crearGafete, crearGafetesRango } from "../api";

interface ValoresFormulario {
  modo: "individual" | "rango";
  numero: string;
  desde: string;
  hasta: string;
}

const numeroValido = (valor: string) => {
  const n = Number(valor);
  return valor.trim() !== "" && Number.isInteger(n) && n > 0;
};

// Validación de UI antes de despachar — la validación real y atómica queda
// en GafeteService (núcleo); esto sólo evita un típo evidente (vacío, no
// numérico) o un rango descomunal por error de tecleo (tope defensivo de
// 200, mismo criterio que la TUI — docs/plan-gafetes.md).
export const esquema = z
  .object({
    modo: z.enum(["individual", "rango"]),
    numero: z.string(),
    desde: z.string(),
    hasta: z.string(),
  })
  .superRefine((valores, ctx) => {
    if (valores.modo === "individual") {
      if (!numeroValido(valores.numero)) {
        ctx.addIssue({
          code: "custom",
          path: ["numero"],
          message: "Ingrese un número de gafete válido",
        });
      }
      return;
    }
    if (!numeroValido(valores.desde)) {
      ctx.addIssue({ code: "custom", path: ["desde"], message: 'Ingrese un "desde" válido' });
      return;
    }
    if (!numeroValido(valores.hasta) || Number(valores.hasta) < Number(valores.desde)) {
      ctx.addIssue({ code: "custom", path: ["hasta"], message: "El rango no es válido" });
      return;
    }
    if (Number(valores.hasta) - Number(valores.desde) > 200) {
      ctx.addIssue({
        code: "custom",
        path: ["hasta"],
        message: "El rango es demasiado grande (máximo 200 a la vez)",
      });
    }
  });

export default function FormularioGafete({
  onGuardado,
  onCerrar,
}: {
  onGuardado: () => void;
  onCerrar: () => void;
}) {
  const {
    register,
    handleSubmit,
    watch,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ValoresFormulario>({
    resolver: zodResolver(esquema),
    defaultValues: { modo: "individual", numero: "", desde: "", hasta: "" },
  });
  const modo = watch("modo");

  async function alGuardar(valores: ValoresFormulario) {
    try {
      if (valores.modo === "individual") {
        await crearGafete(Number(valores.numero));
      } else {
        await crearGafetesRango(Number(valores.desde), Number(valores.hasta));
      }
      onGuardado();
    } catch (error) {
      setError("root", { message: String(error) });
    }
  }

  return (
    <Modal titulo="Nuevo gafete" onCerrar={onCerrar}>
      <form
        onSubmit={handleSubmit(alGuardar)}
        style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}
      >
        <div className="campo" style={{ flexDirection: "row", gap: "1.25rem" }}>
          {(["individual", "rango"] as const).map((opcion) => (
            <label
              key={opcion}
              style={{ display: "flex", alignItems: "center", gap: "0.4rem", color: "var(--texto)" }}
            >
              <input type="radio" value={opcion} {...register("modo")} />
              {opcion === "individual" ? "Individual" : "Rango"}
            </label>
          ))}
        </div>

        {modo === "individual" ? (
          <label className="campo">
            Número de gafete
            <input {...register("numero")} inputMode="numeric" autoFocus />
            {errors.numero && (
              <span style={{ color: "var(--error)" }}>{errors.numero.message}</span>
            )}
          </label>
        ) : (
          <div style={{ display: "flex", gap: "0.75rem" }}>
            <label className="campo" style={{ flex: 1 }}>
              Desde
              <input {...register("desde")} inputMode="numeric" autoFocus />
              {errors.desde && (
                <span style={{ color: "var(--error)" }}>{errors.desde.message}</span>
              )}
            </label>
            <label className="campo" style={{ flex: 1 }}>
              Hasta
              <input {...register("hasta")} inputMode="numeric" />
              {errors.hasta && (
                <span style={{ color: "var(--error)" }}>{errors.hasta.message}</span>
              )}
            </label>
          </div>
        )}

        {errors.root && <p style={{ color: "var(--error)" }}>{errors.root.message}</p>}

        <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
          <button type="button" className="boton" onClick={onCerrar}>
            Cancelar
          </button>
          <button type="submit" className="boton boton-primario" disabled={isSubmitting}>
            {isSubmitting ? "Guardando…" : "Guardar"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
