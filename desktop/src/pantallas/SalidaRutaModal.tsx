import { useEffect, useMemo, useState } from "react";
import type { ChangeEvent } from "react";
import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import {
  FilaListaFlotante,
  ListaFlotante,
  SinResultados,
  useListaFlotante,
  useNavegacionFlechas,
} from "../componentes/ListaFlotante";
import { listarEncargadosRuta, listarRutas, listarVehiculosRuta, registrarSalidaRuta } from "../api";
import type { EncargadoRuta, Ruta, VehiculoRuta } from "../api";
import { fechaYMD } from "../tiempo";

const MAX_RESULTADOS = 6;
const CIERRE_LISTA_MS = 120;

interface ValoresFormulario {
  vehiculo_placa: string;
  vehiculo_numero_unidad: string;
  encargado_nombre: string;
  encargado_codigo_empleado: string;
  numero_ruta: number;
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
    numero_ruta: z
      .number()
      .refine((n) => Number.isInteger(n) && n > 0, "Elija un número de ruta"),
    sub_numero: z.number().int().min(1, "El sub-número debe ser mayor a cero"),
    numero_documento: z.string().min(1, "El número de documento es obligatorio"),
    fecha_documento: z.string().min(1, "La fecha del documento es obligatoria"),
    tiene_correo_autorizacion: z.boolean(),
  })
  .refine((valores) => valores.fecha_documento === hoy() || valores.tiene_correo_autorizacion, {
    message: "El documento no es de hoy -- confirme que cuenta con el correo de autorización",
    path: ["tiene_correo_autorizacion"],
  });

/** Encabezado sutil de sección -- mismo criterio visual que el resto de la
 * app usa para separar bloques dentro de un modal (ver el panel expandido
 * de `NuevoIngresoModal`), pero acá son 3 grupos fijos (vehículo/encargado/
 * documento) en vez de un único panel condicional. */
function TituloSeccion({ children }: { children: string }) {
  return (
    <p
      style={{
        margin: "0 0 0.35rem",
        fontSize: "0.75rem",
        fontWeight: 600,
        textTransform: "uppercase",
        letterSpacing: "0.04em",
        color: "var(--muted)",
      }}
    >
      {children}
    </p>
  );
}

function Seccion({ titulo, children }: { titulo: string; children: React.ReactNode }) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "0.6rem",
        padding: "0.75rem 0.85rem",
        border: "1px solid var(--borde)",
        borderRadius: "var(--radio-chico)",
        background: "var(--campo-fondo)",
      }}
    >
      <TituloSeccion>{titulo}</TituloSeccion>
      {children}
    </div>
  );
}

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
    setValue,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ValoresFormulario>({
    resolver: zodResolver(esquema),
    defaultValues: {
      vehiculo_placa: "",
      vehiculo_numero_unidad: "",
      encargado_nombre: "",
      encargado_codigo_empleado: "",
      numero_ruta: Number.NaN,
      sub_numero: 1,
      numero_documento: "",
      fecha_documento: hoy(),
      tiene_correo_autorizacion: false,
    },
  });

  // `type="number"` deja teclear "e"/"-"/"+" (notación científica) aunque
  // el campo sea un entero positivo -- texto + filtrado en onChange, mismo
  // criterio que FormularioGafete.tsx/IngresoProveedorModal.tsx
  // (docs/pendientes.md, "Auditar máscaras de entrada").
  const registroSubNumero = register("sub_numero", {
    setValueAs: (valor: string) => (valor === "" ? Number.NaN : Number(valor)),
  });
  const alCambiarSubNumero = (evento: ChangeEvent<HTMLInputElement>) => {
    evento.target.value = evento.target.value.replace(/\D/g, "");
    registroSubNumero.onChange(evento);
  };

  const [vehiculos, setVehiculos] = useState<VehiculoRuta[]>([]);
  const [encargados, setEncargados] = useState<EncargadoRuta[]>([]);
  const [rutas, setRutas] = useState<Ruta[]>([]);
  useEffect(() => {
    listarVehiculosRuta().then(setVehiculos).catch(() => {});
    listarEncargadosRuta().then(setEncargados).catch(() => {});
    listarRutas().then(setRutas).catch(() => {});
  }, []);
  const rutasActivas = useMemo(() => rutas.filter((r) => r.activo), [rutas]);

  const placaTexto = watch("vehiculo_placa");
  const numeroUnidad = watch("vehiculo_numero_unidad");
  const encargadoTexto = watch("encargado_nombre");
  const codigoEmpleado = watch("encargado_codigo_empleado");

  const [campoVehiculoEnfocado, setCampoVehiculoEnfocado] = useState(false);
  const [campoEncargadoEnfocado, setCampoEncargadoEnfocado] = useState(false);

  const resultadosVehiculo = useMemo(() => {
    const texto = placaTexto.trim().toLowerCase();
    if (!texto) return [];
    return vehiculos
      .filter(
        (v) =>
          v.placa.toLowerCase().includes(texto) ||
          v.numero_unidad?.toLowerCase().includes(texto),
      )
      .slice(0, MAX_RESULTADOS);
  }, [placaTexto, vehiculos]);

  const resultadosEncargado = useMemo(() => {
    const texto = encargadoTexto.trim().toLowerCase();
    if (!texto) return [];
    return encargados
      .filter(
        (e) =>
          e.nombre.toLowerCase().includes(texto) || e.codigo_empleado.includes(texto),
      )
      .slice(0, MAX_RESULTADOS);
  }, [encargadoTexto, encargados]);

  const listaVehiculoVisible = campoVehiculoEnfocado && resultadosVehiculo.length > 0;
  const listaEncargadoVisible = campoEncargadoEnfocado && resultadosEncargado.length > 0;

  const { campoRef: campoVehiculoRef, posicion: posicionVehiculo } =
    useListaFlotante(listaVehiculoVisible);
  const { campoRef: campoEncargadoRef, posicion: posicionEncargado } =
    useListaFlotante(listaEncargadoVisible);

  function elegirVehiculo(vehiculo: VehiculoRuta) {
    setValue("vehiculo_placa", vehiculo.placa);
    setValue("vehiculo_numero_unidad", vehiculo.numero_unidad ?? "");
    setCampoVehiculoEnfocado(false);
  }

  function elegirEncargado(encargado: EncargadoRuta) {
    setValue("encargado_nombre", encargado.nombre);
    setValue("encargado_codigo_empleado", encargado.codigo_empleado);
    setCampoEncargadoEnfocado(false);
  }

  const {
    resaltado: resaltadoVehiculo,
    setResaltado: setResaltadoVehiculo,
    manejarTecla: manejarTeclaVehiculo,
  } = useNavegacionFlechas(resultadosVehiculo, listaVehiculoVisible, elegirVehiculo);
  const {
    resaltado: resaltadoEncargado,
    setResaltado: setResaltadoEncargado,
    manejarTecla: manejarTeclaEncargado,
  } = useNavegacionFlechas(resultadosEncargado, listaEncargadoVisible, elegirEncargado);

  const fechaDocumento = watch("fecha_documento");
  const documentoRequiereAutorizacion = fechaDocumento !== hoy();

  async function alGuardar(valores: ValoresFormulario) {
    try {
      await registrarSalidaRuta({
        vehiculo_placa: valores.vehiculo_placa.trim(),
        vehiculo_numero_unidad: valores.vehiculo_numero_unidad.trim() || null,
        encargado_nombre: valores.encargado_nombre.trim(),
        encargado_codigo_empleado: valores.encargado_codigo_empleado.trim() || null,
        numero_ruta: valores.numero_ruta,
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
        style={{ display: "flex", flexDirection: "column", gap: "0.65rem", width: "30rem", maxWidth: "100%" }}
      >
        <Seccion titulo="Encargado">
          <div ref={campoEncargadoRef}>
            <label className="campo">
              Nombre o código de empleado
              <input
                {...register("encargado_nombre")}
                autoFocus
                autoComplete="off"
                placeholder="Escriba para buscar en el carnet KOF…"
                onFocus={() => setCampoEncargadoEnfocado(true)}
                onBlur={() => setTimeout(() => setCampoEncargadoEnfocado(false), CIERRE_LISTA_MS)}
                onKeyDown={manejarTeclaEncargado}
              />
            </label>
            {errors.encargado_nombre && (
              <span style={{ color: "var(--error)" }}>{errors.encargado_nombre.message}</span>
            )}
          </div>
          {listaEncargadoVisible && posicionEncargado && (
            <ListaFlotante posicion={posicionEncargado}>
              {resultadosEncargado.length === 0 && <SinResultados />}
              {resultadosEncargado.map((encargado, indice) => (
                <FilaListaFlotante
                  key={encargado.id}
                  resaltada={indice === resaltadoEncargado}
                  onClick={() => elegirEncargado(encargado)}
                  onMouseEnter={() => setResaltadoEncargado(indice)}
                >
                  <span>{encargado.nombre}</span>
                  <span style={{ color: "var(--muted)", fontSize: "0.85rem" }}>
                    #{encargado.codigo_empleado}
                  </span>
                </FilaListaFlotante>
              ))}
            </ListaFlotante>
          )}
          {codigoEmpleado && (
            <p style={{ margin: 0, fontSize: "0.8rem", color: "var(--muted)" }}>
              Código de empleado #{codigoEmpleado}
            </p>
          )}
        </Seccion>

        <Seccion titulo="Documento de carga">
          <div style={{ display: "flex", gap: "0.75rem" }}>
            <label className="campo" style={{ flex: 1 }}>
              N.° de ruta
              <select
                {...register("numero_ruta", { valueAsNumber: true })}
                defaultValue=""
              >
                <option value="" disabled>
                  {rutasActivas.length === 0 ? "Cargando…" : "Elija un número…"}
                </option>
                {rutasActivas.map((ruta) => (
                  <option key={ruta.id} value={ruta.numero}>
                    {ruta.numero}
                  </option>
                ))}
              </select>
              {errors.numero_ruta && (
                <span style={{ color: "var(--error)" }}>{errors.numero_ruta.message}</span>
              )}
            </label>
            <label className="campo" style={{ flex: "0 1 6rem" }}>
              Sub-número
              <input {...registroSubNumero} onChange={alCambiarSubNumero} inputMode="numeric" />
              {errors.sub_numero && (
                <span style={{ color: "var(--error)" }}>{errors.sub_numero.message}</span>
              )}
            </label>
          </div>

          <div style={{ display: "flex", gap: "0.75rem" }}>
            <label className="campo" style={{ flex: 1 }}>
              N.° de documento
              <input {...register("numero_documento")} placeholder="Comprobante de carga" />
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
                padding: "0.5rem 0.65rem",
                border: "1px solid var(--advertencia)",
                borderRadius: "var(--radio-chico)",
              }}
            >
              <input type="checkbox" {...register("tiene_correo_autorizacion")} />
              El documento no es de hoy -- cuento con el correo de autorización
            </label>
          )}
          {errors.tiene_correo_autorizacion && (
            <span style={{ color: "var(--error)" }}>
              {errors.tiene_correo_autorizacion.message}
            </span>
          )}
        </Seccion>

        <Seccion titulo="Vehículo">
          <div ref={campoVehiculoRef}>
            <label className="campo">
              Placa o número de unidad
              <input
                {...register("vehiculo_placa")}
                autoComplete="off"
                placeholder="Escriba para buscar -- las unidades son únicas, no importa cuál use"
                onFocus={() => setCampoVehiculoEnfocado(true)}
                onBlur={() => setTimeout(() => setCampoVehiculoEnfocado(false), CIERRE_LISTA_MS)}
                onKeyDown={manejarTeclaVehiculo}
              />
            </label>
            {errors.vehiculo_placa && (
              <span style={{ color: "var(--error)" }}>{errors.vehiculo_placa.message}</span>
            )}
          </div>
          {listaVehiculoVisible && posicionVehiculo && (
            <ListaFlotante posicion={posicionVehiculo}>
              {resultadosVehiculo.length === 0 && <SinResultados />}
              {resultadosVehiculo.map((vehiculo, indice) => (
                <FilaListaFlotante
                  key={vehiculo.id}
                  resaltada={indice === resaltadoVehiculo}
                  onClick={() => elegirVehiculo(vehiculo)}
                  onMouseEnter={() => setResaltadoVehiculo(indice)}
                >
                  <span>{vehiculo.placa}</span>
                  <span style={{ color: "var(--muted)", fontSize: "0.85rem" }}>
                    {vehiculo.numero_unidad ?? "Sin unidad"}
                  </span>
                </FilaListaFlotante>
              ))}
            </ListaFlotante>
          )}
          {numeroUnidad && (
            <p style={{ margin: 0, fontSize: "0.8rem", color: "var(--muted)" }}>
              Unidad {numeroUnidad}
            </p>
          )}
        </Seccion>

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
