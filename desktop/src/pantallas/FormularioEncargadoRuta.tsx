import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import { actualizarEncargadoRuta, crearEncargadoRuta } from "../api";
import type { EncargadoRuta } from "../api";

interface ValoresFormulario {
  codigo_empleado: string;
  nombre: string;
  activo: boolean;
}

export const esquema = z.object({
  codigo_empleado: z
    .string()
    .regex(/^\d{5,7}$/, "El código de empleado debe tener entre 5 y 7 dígitos"),
  nombre: z.string().min(1, "El nombre es obligatorio"),
  activo: z.boolean(),
});

export default function FormularioEncargadoRuta({
  encargado,
  onGuardado,
  onCerrar,
}: {
  /** Si viene, es edición; si no, alta. */
  encargado?: EncargadoRuta;
  onGuardado: () => void;
  onCerrar: () => void;
}) {
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ValoresFormulario>({
    resolver: zodResolver(esquema),
    defaultValues: {
      codigo_empleado: encargado?.codigo_empleado ?? "",
      nombre: encargado?.nombre ?? "",
      activo: encargado?.activo ?? true,
    },
  });

  async function alGuardar(valores: ValoresFormulario) {
    const datos = {
      codigo_empleado: valores.codigo_empleado.trim(),
      nombre: valores.nombre.trim(),
      activo: valores.activo,
    };
    try {
      if (encargado) {
        await actualizarEncargadoRuta(encargado.id, datos);
      } else {
        await crearEncargadoRuta(datos);
      }
      onGuardado();
    } catch (error) {
      setError("root", { message: String(error) });
    }
  }

  return (
    <Modal titulo={encargado ? "Editar encargado" : "Nuevo encargado"} onCerrar={onCerrar}>
      <form
        onSubmit={handleSubmit(alGuardar)}
        style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}
      >
        <label className="campo">
          Código de empleado
          <input
            {...register("codigo_empleado")}
            autoFocus
            inputMode="numeric"
            placeholder="5 a 7 dígitos, según el carnet"
          />
          {errors.codigo_empleado && (
            <span style={{ color: "var(--error)" }}>{errors.codigo_empleado.message}</span>
          )}
        </label>

        <label className="campo">
          Nombre
          <input {...register("nombre")} />
          {errors.nombre && <span style={{ color: "var(--error)" }}>{errors.nombre.message}</span>}
        </label>

        <label style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <input type="checkbox" {...register("activo")} />
          Activo
        </label>

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
