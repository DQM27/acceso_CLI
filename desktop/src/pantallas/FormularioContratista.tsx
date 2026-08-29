import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import { actualizarContratista, crearContratista, requierePraind } from "../api";
import type { ContratistaResumen, DatosContratista, Empresa, TipoIngreso } from "../api";
import { cedulaSchema, nombreSchema, sanearSoloDigitos, sanearSoloLetras } from "../validacion";

const TIPOS = ["Praind", "InHouse", "PorCorreo", "Swat"] as const;

interface ValoresFormulario {
  cedula: string;
  nombre: string;
  empresa_id: string;
  tipo_ingreso: TipoIngreso;
  fecha_vencimiento_praind: string;
  es_personal_ruta: boolean;
  tiene_acceso: boolean;
}

// La validación real vive en el core (services/contratista_service.rs) — este
// esquema es sólo para dar feedback inmediato sin ida y vuelta al backend.
export const esquema = z
  .object({
    cedula: cedulaSchema,
    nombre: nombreSchema,
    empresa_id: z.string().min(1, "Seleccioná una empresa"),
    tipo_ingreso: z.enum(TIPOS),
    fecha_vencimiento_praind: z.string(),
    es_personal_ruta: z.boolean(),
    tiene_acceso: z.boolean(),
  })
  .refine((datos) => !requierePraind(datos) || datos.fecha_vencimiento_praind !== "", {
    message: "Obligatoria para este tipo de contratista",
    path: ["fecha_vencimiento_praind"],
  });

export default function FormularioContratista({
  contratista,
  empresas,
  onGuardado,
  onCerrar,
}: {
  /** Si viene, es edición; si no, alta. */
  contratista?: ContratistaResumen;
  empresas: Empresa[];
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
    defaultValues: contratista
      ? {
          cedula: contratista.cedula,
          nombre: contratista.nombre,
          empresa_id: String(contratista.empresa_id),
          tipo_ingreso: contratista.tipo_ingreso,
          fecha_vencimiento_praind: contratista.fecha_vencimiento_praind ?? "",
          es_personal_ruta: contratista.es_personal_ruta,
          tiene_acceso: contratista.tiene_acceso,
        }
      : {
          cedula: "",
          nombre: "",
          empresa_id: "",
          tipo_ingreso: "Praind",
          fecha_vencimiento_praind: "",
          es_personal_ruta: false,
          tiene_acceso: true,
        },
  });

  const mostrarPraind = requierePraind(watch());

  async function alGuardar(valores: ValoresFormulario) {
    const datos: DatosContratista = {
      cedula: valores.cedula.trim(),
      nombre: valores.nombre.trim(),
      empresa_id: Number(valores.empresa_id),
      tipo_ingreso: valores.tipo_ingreso,
      fecha_vencimiento_praind:
        mostrarPraind && valores.fecha_vencimiento_praind
          ? valores.fecha_vencimiento_praind
          : null,
      es_personal_ruta: valores.es_personal_ruta,
      tiene_acceso: valores.tiene_acceso,
    };
    try {
      if (contratista) {
        await actualizarContratista(contratista.id, datos);
      } else {
        await crearContratista(datos);
      }
      onGuardado();
    } catch (error) {
      setError("root", { message: String(error) });
    }
  }

  return (
    <Modal titulo={contratista ? "Editar contratista" : "Nuevo contratista"} onCerrar={onCerrar}>
      <form
        onSubmit={handleSubmit(alGuardar)}
        style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}
      >
        <label className="campo">
          Cédula
          <input
            {...register("cedula", {
              onChange: (evento) => {
                evento.target.value = sanearSoloDigitos(evento.target.value);
              },
            })}
            inputMode="numeric"
            disabled={!!contratista}
          />
          {errors.cedula && <span style={{ color: "var(--error)" }}>{errors.cedula.message}</span>}
        </label>

        <label className="campo">
          Nombre
          <input
            {...register("nombre", {
              onChange: (evento) => {
                evento.target.value = sanearSoloLetras(evento.target.value);
              },
            })}
          />
          {errors.nombre && <span style={{ color: "var(--error)" }}>{errors.nombre.message}</span>}
        </label>

        <label className="campo">
          Empresa
          <select {...register("empresa_id")}>
            <option value="">Seleccionar…</option>
            {empresas.map((empresa) => (
              <option key={empresa.id} value={empresa.id}>
                {empresa.nombre}
              </option>
            ))}
          </select>
          {errors.empresa_id && (
            <span style={{ color: "var(--error)" }}>{errors.empresa_id.message}</span>
          )}
        </label>

        <label className="campo">
          Tipo de ingreso
          <select {...register("tipo_ingreso")}>
            {TIPOS.map((tipo) => (
              <option key={tipo} value={tipo}>
                {tipo}
              </option>
            ))}
          </select>
        </label>

        <label
          style={{ display: "flex", alignItems: "center", gap: "0.4rem", color: "var(--texto)" }}
        >
          <input type="checkbox" {...register("es_personal_ruta")} />
          Personal de ruta
        </label>

        <label
          style={{ display: "flex", alignItems: "center", gap: "0.4rem", color: "var(--texto)" }}
        >
          <input type="checkbox" {...register("tiene_acceso")} />
          Con acceso
        </label>

        {mostrarPraind && (
          <label className="campo">
            Fecha de vencimiento PRAIND
            <input type="date" {...register("fecha_vencimiento_praind")} />
            {errors.fecha_vencimiento_praind && (
              <span style={{ color: "var(--error)" }}>
                {errors.fecha_vencimiento_praind.message}
              </span>
            )}
          </label>
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
