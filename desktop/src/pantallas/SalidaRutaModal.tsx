import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import { registrarSalidaRuta } from "../api";
import { fechaYMD } from "../tiempo";

interface ValoresFormulario {
  vehiculo_placa: string;
  vehiculo_numero_unidad: string;
  encargado_nombre: string;
  encargado_codigo_empleado: string;
  numero_ruta: string;
  sub_numero: number;
  numero_documento: string;
  fecha_documento: string;
  tiene_correo_autorizacion: boolean;
}

const hoy = () => fechaYMD(new Date());

/** Espejo de `domain::resultado_salida_ruta::verificar_fecha_documento`
 * (núcleo) -- mismo criterio que el resto de la app (`requierePraind`,
 * `puedeContinuarVisita`): la UI replica la regla para no dejar avanzar un
 * formulario que de todos modos va a fallar al confirmar, pero el backend
 * sigue siendo quien decide de verdad (`docs/planes-implementados/plan-control-rutas.md`,
 * "Bloqueo transitorio por documento vencido"). */
const esquema = z
  .object({
    vehiculo_placa: z.string().min(1, "La placa es obligatoria"),
    vehiculo_numero_unidad: z.string(),
    encargado_nombre: z.string().min(1, "El nombre del encargado es obligatorio"),
    encargado_codigo_empleado: z.string(),
    numero_ruta: z.string().min(1, "El número de ruta es obligatorio"),
    sub_numero: z.number().int().min(1, "El sub-número debe ser mayor a cero"),
    numero_documento: z.string().min(1, "El número de documento es obligatorio"),
    fecha_documento: z.string().min(1, "La fecha del documento es obligatoria"),
    tiene_correo_autorizacion: z.boolean(),
  })
  .refine((valores) => valores.fecha_documento === hoy() || valores.tiene_correo_autorizacion, {
    message: "El documento no es de hoy -- confirme que cuenta con el correo de autorización",
    path: ["tiene_correo_autorizacion"],
  });

export default function SalidaRutaModal({
  onRegistrado,
  onCerrar,
}: {
  onRegistrado: () => void;
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
    defaultValues: {
      vehiculo_placa: "",
      vehiculo_numero_unidad: "",
      encargado_nombre: "",
      encargado_codigo_empleado: "",
      numero_ruta: "",
      sub_numero: 1,
      numero_documento: "",
      fecha_documento: hoy(),
      tiene_correo_autorizacion: false,
    },
  });

  const fechaDocumento = watch("fecha_documento");
  const documentoRequiereAutorizacion = fechaDocumento !== hoy();

  async function alGuardar(valores: ValoresFormulario) {
    try {
      await registrarSalidaRuta({
        vehiculo_placa: valores.vehiculo_placa.trim(),
        vehiculo_numero_unidad: valores.vehiculo_numero_unidad.trim() || null,
        encargado_nombre: valores.encargado_nombre.trim(),
        encargado_codigo_empleado: valores.encargado_codigo_empleado.trim() || null,
        numero_ruta: valores.numero_ruta.trim(),
        sub_numero: valores.sub_numero,
        numero_documento: valores.numero_documento.trim(),
        fecha_documento: valores.fecha_documento,
        tiene_correo_autorizacion: valores.tiene_correo_autorizacion,
      });
      onRegistrado();
    } catch (error) {
      setError("root", { message: String(error) });
    }
  }

  return (
    <Modal titulo="Nueva salida de ruta" onCerrar={onCerrar}>
      <form
        onSubmit={handleSubmit(alGuardar)}
        style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}
      >
        <div style={{ display: "flex", gap: "0.75rem" }}>
          <label className="campo" style={{ flex: 1.4 }}>
            Placa
            <input {...register("vehiculo_placa")} autoFocus placeholder="Ej. C12345" />
            {errors.vehiculo_placa && (
              <span style={{ color: "var(--error)" }}>{errors.vehiculo_placa.message}</span>
            )}
          </label>
          <label className="campo" style={{ flex: 1 }}>
            N.° de unidad
            <input {...register("vehiculo_numero_unidad")} placeholder="Opcional" />
          </label>
        </div>

        <div style={{ display: "flex", gap: "0.75rem" }}>
          <label className="campo" style={{ flex: 1.4 }}>
            Encargado
            <input {...register("encargado_nombre")} placeholder="Nombre del carnet KOF" />
            {errors.encargado_nombre && (
              <span style={{ color: "var(--error)" }}>{errors.encargado_nombre.message}</span>
            )}
          </label>
          <label className="campo" style={{ flex: 1 }}>
            Código de empleado
            <input
              {...register("encargado_codigo_empleado")}
              inputMode="numeric"
              placeholder="Opcional"
            />
          </label>
        </div>

        <div style={{ display: "flex", gap: "0.75rem" }}>
          <label className="campo" style={{ flex: 1 }}>
            N.° de ruta
            <input {...register("numero_ruta")} placeholder="Ej. CRR079" />
            {errors.numero_ruta && (
              <span style={{ color: "var(--error)" }}>{errors.numero_ruta.message}</span>
            )}
          </label>
          <label className="campo" style={{ flex: "0 1 7rem" }}>
            Sub-número
            <input
              type="number"
              min={1}
              {...register("sub_numero", { valueAsNumber: true })}
            />
            {errors.sub_numero && (
              <span style={{ color: "var(--error)" }}>{errors.sub_numero.message}</span>
            )}
          </label>
        </div>

        <div style={{ display: "flex", gap: "0.75rem" }}>
          <label className="campo" style={{ flex: 1 }}>
            N.° de documento (comprobante de carga)
            <input {...register("numero_documento")} />
            {errors.numero_documento && (
              <span style={{ color: "var(--error)" }}>{errors.numero_documento.message}</span>
            )}
          </label>
          <label className="campo" style={{ flex: "0 1 11rem" }}>
            Fecha del documento
            <input type="date" {...register("fecha_documento")} />
          </label>
        </div>

        {documentoRequiereAutorizacion && (
          <label
            style={{
              display: "flex",
              alignItems: "center",
              gap: "0.5rem",
              padding: "0.6rem 0.75rem",
              border: "1px solid var(--borde)",
              borderRadius: "var(--radio-chico)",
              background: "var(--campo-fondo)",
            }}
          >
            <input type="checkbox" {...register("tiene_correo_autorizacion")} />
            El documento no es de hoy -- cuento con el correo de autorización
          </label>
        )}
        {errors.tiene_correo_autorizacion && (
          <span style={{ color: "var(--error)" }}>{errors.tiene_correo_autorizacion.message}</span>
        )}

        {errors.root && <p style={{ color: "var(--error)" }}>{errors.root.message}</p>}

        <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
          <button type="button" className="boton" onClick={onCerrar}>
            Cancelar
          </button>
          <button type="submit" className="boton boton-primario" disabled={isSubmitting}>
            {isSubmitting ? "Registrando…" : "Registrar salida"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
