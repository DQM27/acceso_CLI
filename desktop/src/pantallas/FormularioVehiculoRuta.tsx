import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import { actualizarVehiculoRuta, crearVehiculoRuta } from "../api";
import type { VehiculoRuta } from "../api";

interface ValoresFormulario {
  placa: string;
  numero_unidad: string;
  activo: boolean;
}

export const esquema = z.object({
  placa: z.string().min(1, "La placa es obligatoria"),
  numero_unidad: z.string(),
  activo: z.boolean(),
});

export default function FormularioVehiculoRuta({
  vehiculo,
  onGuardado,
  onCerrar,
}: {
  /** Si viene, es edición; si no, alta. */
  vehiculo?: VehiculoRuta;
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
      placa: vehiculo?.placa ?? "",
      numero_unidad: vehiculo?.numero_unidad ?? "",
      activo: vehiculo?.activo ?? true,
    },
  });

  async function alGuardar(valores: ValoresFormulario) {
    const datos = {
      placa: valores.placa.trim(),
      numero_unidad: valores.numero_unidad.trim() || null,
      activo: valores.activo,
    };
    try {
      if (vehiculo) {
        await actualizarVehiculoRuta(vehiculo.id, datos);
      } else {
        await crearVehiculoRuta(datos);
      }
      onGuardado();
    } catch (error) {
      setError("root", { message: String(error) });
    }
  }

  return (
    <Modal titulo={vehiculo ? "Editar vehículo" : "Nuevo vehículo"} onCerrar={onCerrar}>
      <form
        onSubmit={handleSubmit(alGuardar)}
        style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}
      >
        <label className="campo">
          Placa
          <input {...register("placa")} autoFocus placeholder="Ej. C12345" />
          {errors.placa && <span style={{ color: "var(--error)" }}>{errors.placa.message}</span>}
        </label>

        <label className="campo">
          Número de unidad
          <input
            {...register("numero_unidad")}
            placeholder="Vacío para vehículos de apoyo sin número"
          />
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
