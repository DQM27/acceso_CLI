import { useEffect, useState } from "react";
import { useForm, useWatch } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import {
  actualizarContratista,
  crearContratista,
  reglasFormularioContratista,
  tiposIngresoSeleccionables,
} from "../api";
import type {
  ContratistaResumen,
  DatosContratista,
  Empresa,
  ReglasFormularioContratista,
  TipoIngreso,
} from "../api";
import { sanearSoloDigitos, sanearSoloLetras } from "../validacion";
import { esquema } from "./FormularioContratista.logica";

interface ValoresFormulario {
  cedula: string;
  nombre: string;
  empresa_id: string;
  tipo_ingreso: TipoIngreso;
  fecha_vencimiento_praind: string;
  es_personal_ruta: boolean;
  tiene_acceso: boolean;
}

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
    control,
    setError,
    setValue,
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

  // Los tipos que se pueden elegir los decide el núcleo. El selector se
  // monta recién cuando llegan, así toma el valor del formulario (al editar,
  // el tipo que ya tenía) en vez del primero de la lista.
  const [tiposElegibles, setTiposElegibles] = useState<TipoIngreso[] | null>(null);
  const [errorTipos, setErrorTipos] = useState<string | null>(null);
  useEffect(() => {
    let vigente = true;
    tiposIngresoSeleccionables()
      .then((tipos) => {
        if (vigente) setTiposElegibles(tipos);
      })
      .catch((error) => {
        if (vigente) setErrorTipos(String(error));
      });
    return () => {
      vigente = false;
    };
  }, []);

  const [esPersonalRuta, tipoIngreso, fechaPraind] = useWatch({
    control,
    name: ["es_personal_ruta", "tipo_ingreso", "fecha_vencimiento_praind"],
  });

  // Qué mostrar lo decide el núcleo (`reglas_formulario_contratista`); acá
  // no se replica ninguna regla. Al guardar el núcleo las vuelve a aplicar.
  const [reglas, setReglas] = useState<ReglasFormularioContratista>({
    requiere_praind: true,
    admite_personal_ruta: true,
    aviso_praind: null,
  });
  useEffect(() => {
    let vigente = true;
    reglasFormularioContratista({
      tipo_ingreso: tipoIngreso,
      es_personal_ruta: esPersonalRuta,
      fecha_vencimiento_praind: fechaPraind || null,
    })
      .then((nuevas) => {
        if (vigente) setReglas(nuevas);
      })
      .catch(() => {
        // Sin reglas no se bloquea nada: al guardar decide el núcleo.
      });
    return () => {
      vigente = false;
    };
  }, [tipoIngreso, esPersonalRuta, fechaPraind]);
  useEffect(() => {
    // La casilla se oculta para los tipos que no la admiten; sin esto un
    // `true` que quedó de otro tipo se mandaría igual.
    if (!reglas.admite_personal_ruta && esPersonalRuta) setValue("es_personal_ruta", false);
  }, [reglas.admite_personal_ruta, esPersonalRuta, setValue]);
  const mostrarPraind = reglas.requiere_praind;

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
      es_personal_ruta: reglas.admite_personal_ruta && valores.es_personal_ruta,
      // El alta siempre queda con acceso (lo fija el núcleo); sólo al
      // editar se puede quitar.
      tiene_acceso: contratista ? valores.tiene_acceso : true,
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
            {/* `empresas` (prop) sólo trae activas -- si estamos editando un
                contratista cuya empresa ya se desactivó, se agrega acá como
                única excepción, marcada, para no perderla de la vista ni
                reasignarla en silencio a otra empresa al guardar. */}
            {contratista &&
              !empresas.some((empresa) => empresa.id === contratista.empresa_id) && (
                <option value={contratista.empresa_id}>
                  {contratista.empresa_nombre} (inactiva)
                </option>
              )}
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
          {tiposElegibles ? (
            <select {...register("tipo_ingreso")}>
              {tiposElegibles.map((tipo) => (
                <option key={tipo} value={tipo}>
                  {tipo}
                </option>
              ))}
            </select>
          ) : (
            <select disabled aria-busy={!errorTipos}>
              <option>{errorTipos ? "No se pudieron cargar los tipos" : "Cargando…"}</option>
            </select>
          )}
          {errorTipos && <span style={{ color: "var(--error)" }}>{errorTipos}</span>}
        </label>

        {reglas.admite_personal_ruta && (
          <label
            style={{ display: "flex", alignItems: "center", gap: "0.4rem", color: "var(--texto)" }}
          >
            <input type="checkbox" {...register("es_personal_ruta")} />
            Personal de ruta
          </label>
        )}

        {contratista && (
          <label
            style={{ display: "flex", alignItems: "center", gap: "0.4rem", color: "var(--texto)" }}
          >
            <input type="checkbox" {...register("tiene_acceso")} />
            Con acceso
          </label>
        )}

        {mostrarPraind && (
          <label className="campo">
            Fecha de vencimiento PRAIND
            <input type="date" {...register("fecha_vencimiento_praind")} />
            {reglas.aviso_praind && (
              <span style={{ color: "var(--error)" }}>{reglas.aviso_praind}</span>
            )}
          </label>
        )}

        {errors.root && <p style={{ color: "var(--error)" }}>{errors.root.message}</p>}

        <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
          <button type="button" className="boton" onClick={onCerrar}>
            Cancelar
          </button>
          <button type="submit" className="boton boton-primario" disabled={isSubmitting || (mostrarPraind && reglas.aviso_praind !== null)}
          >
            {isSubmitting ? "Guardando…" : "Guardar"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
