import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import Modal from "../componentes/Modal";
import { crearContratista, crearEmpresa, listarEmpresas } from "../api/contratistas";
import type { Empresa, TipoIngreso } from "../api/contratistas";
import { sanearSoloDigitos, sanearSoloLetras } from "../validacion";
import { mensajeError } from "../mensajeError";
import { TIPOS_INGRESO, errorAntesDeEnviar, pidePraind } from "./FormularioContratista.logica";

/**
 * Alta de un contratista desde el panel. Sirve para dos cosas: registrar a un
 * contratista de verdad y, sobre todo, **negar el acceso a alguien que nunca
 * fue contratista** (un proveedor, por ejemplo): se lo da de alta con el acceso
 * denegado y ninguna puerta lo deja entrar (ver
 * docs/features-futuras/plan-veto-por-persona.md).
 *
 * Aquí no se replica ninguna regla: cédula, nombre, PRAIND y cédula repetida
 * los valida la base al guardar (migración `panel_crea_contratistas`) y su
 * mensaje se muestra tal cual. Igual que en escritorio, el nombre y la cédula
 * solo admiten lo válido mientras se escribe.
 *
 * La empresa es obligatoria porque los equipos descartan un contratista sin una
 * empresa que puedan resolver, y entonces el bloqueo no les llegaría.
 */
export default function FormularioContratista({
  onGuardado,
  onCerrar,
}: {
  onGuardado: () => void;
  onCerrar: () => void;
}) {
  const clienteConsultas = useQueryClient();
  const { data: empresas = [] } = useQuery({ queryKey: ["empresas"], queryFn: listarEmpresas });

  const [cedula, setCedula] = useState("");
  const [nombre, setNombre] = useState("");
  const [empresaId, setEmpresaId] = useState("");
  const [tipo, setTipo] = useState<TipoIngreso>("PRAIND");
  const [praind, setPraind] = useState("");
  const [denegado, setDenegado] = useState(false);
  const [enviando, setEnviando] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [creandoEmpresa, setCreandoEmpresa] = useState(false);
  const [nombreEmpresa, setNombreEmpresa] = useState("");
  const [guardandoEmpresa, setGuardandoEmpresa] = useState(false);

  const mostrarPraind = pidePraind(tipo, !denegado);

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
    const problema = errorAntesDeEnviar({ empresaId });
    if (problema) {
      setError(problema);
      return;
    }
    setEnviando(true);
    setError(null);
    try {
      const creado = await crearContratista({
        cedula: cedula.trim(),
        nombre: nombre.trim(),
        empresa_id: empresaId,
        tipo_ingreso: tipo,
        fecha_vencimiento_praind: mostrarPraind && praind ? praind : null,
        con_acceso: !denegado,
      });
      toast.success(
        denegado
          ? `${creado.nombre} registrado con el acceso denegado.`
          : `${creado.nombre} registrado.`,
      );
      onGuardado();
    } catch (fallo) {
      setError(mensajeError(fallo));
    } finally {
      setEnviando(false);
    }
  }

  return (
    <Modal titulo="Nuevo contratista" onCerrar={onCerrar}>
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
            onChange={(evento) => setNombre(sanearSoloLetras(evento.target.value))}
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
            {TIPOS_INGRESO.map(({ valor, etiqueta }) => (
              <option key={valor} value={valor}>
                {etiqueta}
              </option>
            ))}
          </select>
        </label>

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
            Crear con el acceso denegado
          </label>
          <p className="m-0 text-[0.8rem] text-muted">
            Úselo para negar el acceso a alguien que no es contratista, por ejemplo un proveedor:
            queda registrado y ninguna puerta lo deja entrar.
          </p>
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
