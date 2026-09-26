import type { ChangeEvent, CSSProperties } from "react";
import { useForm, useWatch } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import { crearGafete, crearGafetesRango } from "../api";
import { esquema, TIPOS, resumenCreacion } from "./FormularioGafete.logica";
import type { ValoresFormulario } from "./FormularioGafete.logica";

/**
 * Rediseño 2026-09-24 (pedido del usuario, mismo espíritu que los modales
 * de KOF y proveedores): el tipo se elige con tarjetas de ícono en vez de un
 * `<select>`, "uno / rango" con el control segmentado deslizante en vez de
 * radios, y una línea de resumen dice exactamente qué se va a crear antes
 * de confirmar. Se cierra con la ✕ o Esc, como los demás modales de alta.
 */
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
    control,
    setValue,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ValoresFormulario>({
    resolver: zodResolver(esquema),
    defaultValues: { modo: "individual", tipo: "contratista", numero: "", desde: "", hasta: "" },
  });
  const valores = useWatch({ control }) as ValoresFormulario;
  const resumen = resumenCreacion(valores);

  // Filtra caracteres no numéricos al tipear, mismo criterio que el
  // buscador de Gafetes.tsx -- evita que se pueda siquiera escribir una
  // letra en vez de sólo rechazarla al enviar (docs/pendientes.md, "Auditar
  // máscaras de entrada").
  const registroNumero = register("numero");
  const registroDesde = register("desde");
  const registroHasta = register("hasta");
  const soloDigitos =
    (registro: { onChange: (evento: ChangeEvent<HTMLInputElement>) => void }) =>
    (evento: ChangeEvent<HTMLInputElement>) => {
      evento.target.value = evento.target.value.replace(/\D/g, "");
      registro.onChange(evento);
    };

  async function alGuardar(datos: ValoresFormulario) {
    try {
      if (datos.modo === "individual") {
        await crearGafete(Number(datos.numero), datos.tipo);
      } else {
        await crearGafetesRango(Number(datos.desde), Number(datos.hasta), datos.tipo);
      }
      onGuardado();
    } catch (error) {
      setError("root", { message: String(error) });
    }
  }

  const errorNumeros = errors.numero?.message ?? errors.desde?.message ?? errors.hasta?.message;
  const modos = [
    { valor: "individual", texto: "Uno" },
    { valor: "rango", texto: "Rango" },
  ] as const;
  const indiceModo = valores.modo === "individual" ? 0 : 1;

  return (
    <Modal titulo="Nuevo gafete" onCerrar={onCerrar}>
      <form
        onSubmit={handleSubmit(alGuardar)}
        style={{ display: "flex", flexDirection: "column", gap: "1.1rem" }}
      >
        <div className="campo">
          Tipo de gafete
          <div className="opciones-tarjeta" role="radiogroup" aria-label="Tipo de gafete">
            {TIPOS.map(({ valor, etiqueta, Icono }) => (
              <button
                key={valor}
                type="button"
                role="radio"
                aria-checked={valores.tipo === valor}
                className="opcion-tarjeta"
                onClick={() => setValue("tipo", valor)}
              >
                <Icono size={20} strokeWidth={1.8} aria-hidden="true" />
                {etiqueta}
              </button>
            ))}
          </div>
        </div>

        <div className="campo">
          Números
          <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
            <div
              className="segmentado segmentado-deslizante"
              role="group"
              aria-label="Cantidad"
              style={{ "--cantidad": 2, "--indice": indiceModo, flexShrink: 0 } as CSSProperties}
            >
              <span className="segmentado-indicador" aria-hidden="true" />
              {modos.map(({ valor, texto }) => (
                <button
                  key={valor}
                  type="button"
                  aria-pressed={valores.modo === valor}
                  onClick={() => setValue("modo", valor)}
                  style={{ padding: "0 0.9rem" }}
                >
                  {texto}
                </button>
              ))}
            </div>

            {valores.modo === "individual" ? (
              <input
                {...registroNumero}
                onChange={soloDigitos(registroNumero)}
                inputMode="numeric"
                autoFocus
                autoComplete="off"
                placeholder="Número"
                aria-label="Número de gafete"
                style={{ flex: 1 }}
              />
            ) : (
              <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flex: 1 }}>
                <input
                  {...registroDesde}
                  onChange={soloDigitos(registroDesde)}
                  inputMode="numeric"
                  autoFocus
                  autoComplete="off"
                  placeholder="Desde"
                  aria-label="Desde"
                  style={{ flex: 1, minWidth: 0 }}
                />
                <span style={{ color: "var(--muted)" }}>a</span>
                <input
                  {...registroHasta}
                  onChange={soloDigitos(registroHasta)}
                  inputMode="numeric"
                  autoComplete="off"
                  placeholder="Hasta"
                  aria-label="Hasta"
                  style={{ flex: 1, minWidth: 0 }}
                />
              </div>
            )}
          </div>
        </div>

        {errorNumeros ? (
          <p className="login-error" role="alert" style={{ margin: 0 }}>
            {errorNumeros}
          </p>
        ) : (
          resumen.frase && (
            <p style={{ margin: 0, color: "var(--muted)", fontSize: "0.9rem" }}>{resumen.frase}</p>
          )
        )}

        {errors.root && (
          <p className="login-error" role="alert" style={{ margin: 0 }}>
            {errors.root.message}
          </p>
        )}

        <div style={{ display: "flex", justifyContent: "flex-end" }}>
          <button type="submit" className="boton boton-primario" disabled={isSubmitting}>
            {isSubmitting ? "Creando…" : resumen.boton}
          </button>
        </div>
      </form>
    </Modal>
  );
}
