import { useRef, useState } from "react";
import type { ReactNode } from "react";
import { X } from "lucide-react";
import Modal from "../componentes/Modal";
import { comprobarConteoGafetes, leerNumeroConteoGafetes } from "../api";
import type { ResultadoConteo, TipoGafeteEntrada } from "../api";
import { avisoLectura, avisoPrevio, parsearNumero, textoNumero } from "./ConteoGafetesModal.logica";
import type { AvisoNumero, NivelAviso } from "./ConteoGafetesModal.logica";

const COLOR_AVISO: Record<NivelAviso, string> = {
  bien: "var(--exito)",
  atencion: "var(--advertencia)",
  error: "var(--error)",
};

/**
 * Conteo físico de gafetes (pedido del usuario, 2026-11-16): el operador va
 * tomando los gafetes en el orden en que están, escribe cada número y pulsa
 * Enter; el campo se limpia y espera el siguiente. Al escribirlo se le avisa
 * si ya lo había puesto, si no existe, si figura perdido o si figura
 * asignado a alguien adentro (en esos dos, primero que revise la
 * digitación). "Comprobar" compara lo anotado con lo que debería haber. Nada
 * se guarda: lo que haya que corregir se hace desde la lista de gafetes, con
 * su reporte.
 */
export default function ConteoGafetesModal({
  tipo,
  etiquetaTipo,
  onCerrar,
}: {
  tipo: TipoGafeteEntrada;
  etiquetaTipo: string;
  onCerrar: () => void;
}) {
  const [texto, setTexto] = useState("");
  const [anotados, setAnotados] = useState<number[]>([]);
  const [aviso, setAviso] = useState<AvisoNumero | null>(null);
  const [resultado, setResultado] = useState<ResultadoConteo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [ocupado, setOcupado] = useState(false);
  const campoRef = useRef<HTMLInputElement>(null);

  async function anotar() {
    if (ocupado) return;
    const previo = avisoPrevio(texto, anotados);
    const numero = parsearNumero(texto);
    if (previo || numero === null) {
      setAviso(previo);
      setTexto("");
      return;
    }
    setOcupado(true);
    setError(null);
    try {
      const lectura = await leerNumeroConteoGafetes(tipo, numero);
      const nuevo = avisoLectura(numero, lectura);
      setAviso(nuevo);
      if (nuevo.anotar) {
        setAnotados((actuales) => [...actuales, numero]);
        setResultado(null);
      }
      setTexto("");
    } catch (error) {
      setError(String(error));
    } finally {
      setOcupado(false);
      campoRef.current?.focus();
    }
  }

  function quitar(numero: number) {
    setAnotados((actuales) => actuales.filter((n) => n !== numero));
    setResultado(null);
    campoRef.current?.focus();
  }

  async function comprobar() {
    setOcupado(true);
    setError(null);
    try {
      setResultado(await comprobarConteoGafetes(tipo, anotados));
    } catch (error) {
      setError(String(error));
    } finally {
      setOcupado(false);
    }
  }

  function empezarDeNuevo() {
    setAnotados([]);
    setResultado(null);
    setAviso(null);
    setTexto("");
    campoRef.current?.focus();
  }

  return (
    <Modal titulo={`Conteo de gafetes · ${etiquetaTipo}`} onCerrar={onCerrar} ancho="40rem">
      <div style={{ display: "flex", flexDirection: "column", gap: "0.9rem" }}>
        <p style={{ margin: 0, color: "var(--muted)" }}>
          Tome los gafetes uno por uno, en el orden en que estén: escriba el número, pulse Enter
          y póngalo aparte. Al terminar, pulse «Comprobar».
        </p>

        <form
          style={{ display: "flex", gap: "0.5rem", alignItems: "flex-end" }}
          onSubmit={(evento) => {
            evento.preventDefault();
            anotar();
          }}
        >
          <label className="campo" style={{ flex: 1 }}>
            Número de gafete
            <input
              ref={campoRef}
              value={texto}
              onChange={(evento) => setTexto(evento.target.value.replace(/\D/g, ""))}
              inputMode="numeric"
              autoFocus
              autoComplete="off"
              placeholder="Número y Enter…"
            />
          </label>
          <button type="submit" className="boton" disabled={ocupado || !texto.trim()}>
            Anotar
          </button>
        </form>

        {aviso && (
          <p role="status" style={{ margin: 0, color: COLOR_AVISO[aviso.nivel], fontWeight: 500 }}>
            {aviso.texto}
          </p>
        )}
        {error && (
          <p className="login-error" role="alert">
            {error}
          </p>
        )}

        <div>
          <p style={{ margin: "0 0 0.4rem", color: "var(--muted)" }}>
            Anotados: <strong style={{ color: "var(--texto)" }}>{anotados.length}</strong>
          </p>
          {anotados.length > 0 && (
            <div style={{ display: "flex", flexWrap: "wrap", gap: "0.35rem" }}>
              {anotados.map((numero) => (
                <span
                  key={numero}
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: "0.2rem",
                    padding: "0.1rem 0.25rem 0.1rem 0.5rem",
                    border: "1px solid var(--borde-fuerte)",
                    borderRadius: "var(--radio-chico)",
                    fontVariantNumeric: "tabular-nums",
                  }}
                >
                  {textoNumero(numero)}
                  <button
                    type="button"
                    aria-label={`Quitar el ${textoNumero(numero)}`}
                    title="Quitar"
                    onClick={() => quitar(numero)}
                    style={{
                      display: "inline-flex",
                      border: "none",
                      background: "none",
                      color: "var(--muted)",
                      cursor: "pointer",
                      padding: "0.1rem",
                    }}
                  >
                    <X size={12} aria-hidden="true" />
                  </button>
                </span>
              ))}
            </div>
          )}
        </div>

        {resultado && <Resultado resultado={resultado} />}

        <div style={{ display: "flex", gap: "0.5rem", justifyContent: "flex-end" }}>
          <button type="button" className="boton" disabled={ocupado} onClick={empezarDeNuevo}>
            Empezar de nuevo
          </button>
          <button type="button" className="boton" onClick={onCerrar}>
            Cerrar
          </button>
          <button
            type="button"
            className="boton boton-primario"
            disabled={ocupado}
            onClick={comprobar}
          >
            Comprobar
          </button>
        </div>
      </div>
    </Modal>
  );
}

function Resultado({ resultado }: { resultado: ResultadoConteo }) {
  const lista = (numeros: number[]) => numeros.map(textoNumero).join(", ");
  const completo = resultado.faltantes.length === 0;
  return (
    <div
      className="tarjeta"
      style={{ padding: "0.75rem 0.9rem", display: "flex", flexDirection: "column", gap: "0.6rem" }}
    >
      <p style={{ margin: 0, fontWeight: 600, color: completo ? "var(--exito)" : "var(--error)" }}>
        {completo
          ? `Están los ${resultado.esperados} gafetes disponibles.`
          : `Faltan ${resultado.faltantes.length} de los ${resultado.esperados} gafetes disponibles.`}
      </p>
      {!completo && (
        <Seccion titulo="Faltan (no se contaron)" color="var(--error)">
          {lista(resultado.faltantes)}
          <Nota>
            Revise esos números exactos. Si no aparecen, repórtelos perdidos desde la lista de
            gafetes.
          </Nota>
        </Seccion>
      )}
      {resultado.en_uso_contados.length > 0 && (
        <Seccion titulo="Se contaron, pero figuran asignados a alguien adentro" color="var(--advertencia)">
          {resultado.en_uso_contados.map((g) => `${textoNumero(g.numero)} (${g.nombre})`).join(", ")}
          <Nota>
            Revise que el número esté bien digitado. Si lo está, falta registrar la salida de esa
            persona.
          </Nota>
        </Seccion>
      )}
      {resultado.perdidos_contados.length > 0 && (
        <Seccion titulo="Se contaron, pero figuran perdidos" color="var(--advertencia)">
          {lista(resultado.perdidos_contados)}
          <Nota>
            Revise que el número esté bien digitado. Si lo está, recupérelo desde la lista de
            gafetes para que se pueda volver a asignar.
          </Nota>
        </Seccion>
      )}
      {resultado.en_uso.length > 0 && (
        <Seccion titulo="Asignados a personas adentro (no se cuentan)">
          {resultado.en_uso.map((g) => `${textoNumero(g.numero)} (${g.nombre})`).join(", ")}
        </Seccion>
      )}
      {resultado.perdidos.length > 0 && (
        <Seccion titulo="Perdidos (no se cuentan)">{lista(resultado.perdidos)}</Seccion>
      )}
      {resultado.no_existen.length > 0 && (
        <Seccion titulo="No existen en el catálogo" color="var(--error)">
          {lista(resultado.no_existen)}
        </Seccion>
      )}
    </div>
  );
}

function Seccion({ titulo, color, children }: { titulo: string; color?: string; children: ReactNode }) {
  return (
    <div>
      <p style={{ margin: "0 0 0.15rem", color: color ?? "var(--muted)", fontSize: "var(--tam-sm)" }}>
        {titulo}
      </p>
      <div style={{ color: "var(--texto)", fontVariantNumeric: "tabular-nums" }}>{children}</div>
    </div>
  );
}

function Nota({ children }: { children: ReactNode }) {
  return (
    <p style={{ margin: "0.2rem 0 0", color: "var(--muted)", fontSize: "var(--tam-sm)" }}>{children}</p>
  );
}
