import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import Modal from "../componentes/Modal";
import { crearContratista, crearEmpresa, editarContratista, listarEmpresas } from "../api/contratistas";
import type { ContratistaConEstado, Empresa, TipoIngreso } from "../api/contratistas";
import { sanearSoloDigitos, sanearSoloLetras } from "../validacion";
import { mensajeError } from "../mensajeError";
import { useEstadoReglas, reglas } from "../reglas";
import { admitePersonalRuta, errorAntesDeEnviar, pidePraind, tiposIngreso } from "./FormularioContratista.logica";

/**
 * Alta y edición de un contratista desde el panel (con `contratista`, edita
 * ese). El alta sirve para dos cosas: registrar a un
 * contratista de verdad y, sobre todo, **negar el acceso a alguien que nunca
 * fue contratista** (un proveedor, por ejemplo): se lo da de alta con el acceso
 * denegado y ninguna puerta lo deja entrar (ver
 * docs/features-futuras/plan-veto-por-persona.md).
 *
 * Aquí no se replica ninguna regla: las de criterio (cédula, nombre, tipo,
 * PRAIND) las aporta el núcleo vía WebAssembly (`src/reglas`) para avisar
 * antes de enviar, y la Edge Function `admin-crear-contratista` las vuelve a
 * aplicar con el mismo código al guardar, junto con las que necesitan datos
 * (empresa existente, cédula repetida). Su mensaje se muestra tal cual. Igual
 * que en escritorio, el nombre y la cédula solo admiten lo válido mientras se
 * escribe.
 *
 * La empresa es obligatoria porque los equipos descartan un contratista sin una
 * empresa que puedan resolver, y entonces el bloqueo no les llegaría.
 *
 * El formulario necesita las reglas cargadas (deciden los tipos y cuándo se pide
 * PRAIND). Casi siempre ya lo están al abrirlo; si no, espera.
 */
interface Props {
  /** Si viene, se edita este contratista; si no, es un alta. */
  contratista?: ContratistaConEstado;
  onGuardado: () => void;
  onCerrar: () => void;
}

export default function FormularioContratista(props: Props) {
  const estadoReglas = useEstadoReglas();
  if (estadoReglas === "lista") return <FormularioConReglas {...props} />;
  return (
    <Modal titulo={props.contratista ? "Editar contratista" : "Nuevo contratista"} onCerrar={props.onCerrar}>
      {estadoReglas === "cargando" ? (
        <p className="m-0 text-muted">Cargando…</p>
      ) : (
        <p className="login-error" role="alert">
          No se pudieron cargar las reglas del panel. Revise la conexión y recargue la página.
        </p>
      )}
      <div className="mt-3 flex justify-end">
        <button type="button" className="boton" onClick={props.onCerrar}>
          Cerrar
        </button>
      </div>
    </Modal>
  );
}

function FormularioConReglas({ contratista, onGuardado, onCerrar }: Props) {
  const clienteConsultas = useQueryClient();
  const { data: empresas = [] } = useQuery({ queryKey: ["empresas"], queryFn: listarEmpresas });

  // Al editar arranca con lo guardado; el tipo que ya tenía se ofrece aunque
  // esté retirado (ver `tiposIngreso`).
  const tipoGuardado = (contratista?.tipo_ingreso ?? undefined) as TipoIngreso | undefined;
  const [cedula, setCedula] = useState(contratista?.identificacion ?? "");
  const [nombre, setNombre] = useState(contratista?.nombre ?? "");
  const [empresaId, setEmpresaId] = useState(contratista?.empresa_id ?? "");
  const [tipo, setTipo] = useState<TipoIngreso>(tipoGuardado ?? "PRAIND");
  const [praind, setPraind] = useState(contratista?.fecha_vencimiento_praind ?? "");
  const [personalRuta, setPersonalRuta] = useState(contratista?.es_personal_ruta ?? false);
  const [denegado, setDenegado] = useState(contratista ? !contratista.activo : false);
  const [enviando, setEnviando] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [creandoEmpresa, setCreandoEmpresa] = useState(false);
  const [nombreEmpresa, setNombreEmpresa] = useState("");
  const [guardandoEmpresa, setGuardandoEmpresa] = useState(false);

  // La casilla sólo existe para los tipos que la admiten: si se cambia a otro
  // tipo, no se manda un `true` que quedó del anterior.
  const mostrarPersonalRuta = admitePersonalRuta(tipo);
  const esPersonalRuta = mostrarPersonalRuta && personalRuta;
  const mostrarPraind = pidePraind(tipo, !denegado, esPersonalRuta);
  const anterior = contratista
    ? {
        tipo_ingreso: tipoGuardado ?? tipo,
        es_personal_ruta: contratista.es_personal_ruta ?? false,
        fecha_vencimiento_praind: contratista.fecha_vencimiento_praind,
      }
    : undefined;

  async function alCrearEmpresa() {
    setGuardandoEmpresa(true);
    setError(null);
    try {
      const empresa = await crearEmpresa(nombreEmpresa);
      // Aparece en la lista sin esperar a otra consulta, y queda elegida.
      clienteConsultas.setQueryData<Empresa[]>(["empresas"], (actual = []) =>
        actual.some((e) => e.id === empresa.id)
          ? actual
          : [...actual, empresa].sort((a, b) => a.nombre.localeCompare(b.nombre)),
      );
      setEmpresaId(empresa.id);
      setCreandoEmpresa(false);
      setNombreEmpresa("");
    } catch (fallo) {
      setError(mensajeError(fallo));
    } finally {
      setGuardandoEmpresa(false);
    }
  }

  async function alEnviar(evento: React.FormEvent) {
    evento.preventDefault();
    const problema = errorAntesDeEnviar({
      empresaId,
      cedula,
      nombre,
      tipo,
      praind: mostrarPraind && praind ? praind : null,
      conAcceso: !denegado,
      personalRuta: esPersonalRuta,
      anterior,
    });
    if (problema) {
      setError(problema);
      return;
    }
    setEnviando(true);
    setError(null);
    try {
      const datos = {
        cedula: cedula.trim(),
        nombre: nombre.trim(),
        empresa_id: empresaId,
        tipo_ingreso: tipo,
        fecha_vencimiento_praind: mostrarPraind && praind ? praind : null,
        con_acceso: !denegado,
        es_personal_ruta: esPersonalRuta,
      };
      if (contratista) {
        const editado = await editarContratista(contratista.id, datos);
        toast.success(`${editado.nombre} actualizado.`);
      } else {
        const creado = await crearContratista(datos);
        toast.success(
          denegado
            ? `${creado.nombre} registrado con el acceso denegado.`
            : `${creado.nombre} registrado.`,
        );
      }
      onGuardado();
    } catch (fallo) {
      setError(mensajeError(fallo));
    } finally {
      setEnviando(false);
    }
  }

  return (
    <Modal titulo={contratista ? "Editar contratista" : "Nuevo contratista"} onCerrar={onCerrar}>
      <form onSubmit={alEnviar} className="flex flex-col gap-3">
        <label className="campo">
          Cédula
          <input
            required
            autoFocus
            inputMode="numeric"
            value={cedula}
            disabled={enviando}
            onChange={(evento) => setCedula(sanearSoloDigitos(evento.target.value))}
          />
        </label>

        <label className="campo">
          Nombre
          <input
            required
            value={nombre}
            disabled={enviando}
            onChange={(evento) => setNombre(reglas.nombreMientrasSeEscribe(sanearSoloLetras(evento.target.value)))}
          />
        </label>

        <label className="campo">
          Empresa
          {creandoEmpresa ? (
            <div className="flex gap-2">
              <input
                autoFocus
                className="flex-1"
                placeholder="Nombre de la empresa"
                value={nombreEmpresa}
                disabled={guardandoEmpresa}
                onChange={(evento) => setNombreEmpresa(evento.target.value)}
              />
              <button
                type="button"
                className="boton boton-primario"
                disabled={guardandoEmpresa || nombreEmpresa.trim() === ""}
                onClick={alCrearEmpresa}
              >
                {guardandoEmpresa ? "Creando…" : "Crear"}
              </button>
              <button
                type="button"
                className="boton"
                disabled={guardandoEmpresa}
                onClick={() => setCreandoEmpresa(false)}
              >
                Cancelar
              </button>
            </div>
          ) : (
            <div className="flex gap-2">
              <select
                className="flex-1"
                value={empresaId}
                disabled={enviando}
                onChange={(evento) => setEmpresaId(evento.target.value)}
              >
                <option value="">Seleccionar…</option>
                {empresas.map((empresa) => (
                  <option key={empresa.id} value={empresa.id}>
                    {empresa.nombre}
                  </option>
                ))}
              </select>
              <button
                type="button"
                className="boton"
                title="Crear una empresa nueva"
                aria-label="Crear una empresa nueva"
                disabled={enviando}
                onClick={() => setCreandoEmpresa(true)}
              >
                +
              </button>
            </div>
          )}
        </label>

        <label className="campo">
          Tipo de ingreso
          <select
            value={tipo}
            disabled={enviando}
            onChange={(evento) => setTipo(evento.target.value as TipoIngreso)}
          >
            {tiposIngreso(tipoGuardado).map(({ valor, etiqueta }) => (
              <option key={valor} value={valor}>
                {etiqueta}
              </option>
            ))}
          </select>
        </label>

        {mostrarPersonalRuta && (
          <label className="flex items-center gap-[0.4rem] text-texto">
            <input
              type="checkbox"
              checked={personalRuta}
              disabled={enviando}
              onChange={(evento) => setPersonalRuta(evento.target.checked)}
            />
            Personal de ruta
          </label>
        )}

        {mostrarPraind && (
          <label className="campo">
            Fecha de vencimiento PRAIND
            <input
              type="date"
              value={praind}
              disabled={enviando}
              onChange={(evento) => setPraind(evento.target.value)}
            />
          </label>
        )}

        <div className="flex flex-col gap-1">
          <label className="flex items-center gap-[0.4rem] text-texto">
            <input
              type="checkbox"
              checked={denegado}
              disabled={enviando}
              onChange={(evento) => setDenegado(evento.target.checked)}
            />
            {contratista ? "Acceso denegado" : "Crear con el acceso denegado"}
          </label>
          {!contratista && (
            <p className="m-0 text-[0.8rem] text-muted">
              Úselo para negar el acceso a alguien que no es contratista, por ejemplo un proveedor:
              queda registrado y ninguna puerta lo deja entrar.
            </p>
          )}
        </div>

        {error && (
          <p className="login-error" role="alert">
            {error}
          </p>
        )}

        <div className="flex justify-end gap-2">
          <button type="button" className="boton" disabled={enviando} onClick={onCerrar}>
            Cancelar
          </button>
          <button type="submit" className="boton boton-primario" disabled={enviando}>
            {enviando ? "Guardando…" : "Guardar"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
