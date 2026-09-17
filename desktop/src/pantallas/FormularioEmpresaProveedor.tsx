import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import { crearEmpresaProveedor } from "../api/proveedores";

interface ValoresFormulario {
  nombre: string;
}

// Sólo alta -- a diferencia de `FormularioEmpresa`, el núcleo no expone
// renombrar una empresa proveedora (ver docs/features-futuras/plan-control-proveedores.md,
// "mismo CRUD mínimo... crear, buscar, listar, activar/desactivar"). Activar/
// desactivar se hace inline en la grilla, mismo patrón que `Empresas.tsx`.
export const esquema = z.object({
  nombre: z.string().min(1, "El nombre es obligatorio"),
});

export default function FormularioEmpresaProveedor({
  onGuardado,
  onCerrar,
}: {
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
    defaultValues: { nombre: "" },
  });

  async function alGuardar(valores: ValoresFormulario) {
    try {
      await crearEmpresaProveedor(valores.nombre.trim());
      onGuardado();
    } catch (error) {
      setError("root", { message: String(error) });
    }
  }

  return (
    <Modal titulo="Nueva empresa proveedora" onCerrar={onCerrar}>
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
