import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import { actualizarEmpresa, crearEmpresa } from "../api";
import type { EmpresaResumen } from "../api";

interface ValoresFormulario {
  nombre: string;
}

// Sin restricción de caracteres más allá de "no vacío" — a diferencia del
// nombre de un contratista, el de una empresa puede tener números o símbolos
// (S.A., 3M, etc.), no tiene sentido restringirlo.
const esquema = z.object({
  nombre: z.string().min(1, "El nombre es obligatorio"),
});

export default function FormularioEmpresa({
  empresa,
  onGuardado,
  onCerrar,
}: {
  /** Si viene, es edición; si no, alta. */
  empresa?: EmpresaResumen;
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
    defaultValues: { nombre: empresa?.nombre ?? "" },
  });

  async function alGuardar(valores: ValoresFormulario) {
    try {
      if (empresa) {
        await actualizarEmpresa(empresa.id, valores.nombre.trim());
      } else {
        await crearEmpresa(valores.nombre.trim());
      }
      onGuardado();
    } catch (error) {
      setError("root", { message: String(error) });
    }
  }

  return (
    <Modal titulo={empresa ? "Editar empresa" : "Nueva empresa"} onCerrar={onCerrar}>
      <form
        onSubmit={handleSubmit(alGuardar)}
        style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}
      >
        <label className="campo">
          Nombre
          <input {...register("nombre")} autoFocus />
          {errors.nombre && <span style={{ color: "var(--error)" }}>{errors.nombre.message}</span>}
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
