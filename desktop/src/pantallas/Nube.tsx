import { useEffect, useState } from "react";
import { toast } from "sonner";
import PantallaEncabezado from "../componentes/PantallaEncabezado";
import {
  guardarSecretoDispositivo,
  secretoDispositivoGuardado,
  sincronizarConNube,
} from "../api";
import type { ResumenSincronizacion } from "../api";

/**
 * Pantalla exclusiva de Root (`Operacion::GestionarNube`, ver `App.tsx`) --
 * ni Administrador la ve: el secreto de acá es la identidad de todo el
 * equipo ante el receptor en la nube (`docs/plan-persistencia-nube.md`), no
 * una preferencia de la app.
 */
export default function Nube() {
  const [configurado, setConfigurado] = useState<boolean | null>(null);
  const [secreto, setSecreto] = useState("");
  const [guardando, setGuardando] = useState(false);
  const [sincronizando, setSincronizando] = useState(false);
  const [ultimoResumen, setUltimoResumen] = useState<ResumenSincronizacion | null>(null);

  function cargarEstado() {
    secretoDispositivoGuardado()
      .then(setConfigurado)
      .catch((error) => toast.error(String(error)));
  }

  useEffect(cargarEstado, []);

  async function guardar() {
    const valor = secreto.trim();
    if (!valor) return;
    setGuardando(true);
    try {
      await guardarSecretoDispositivo(valor);
      toast.success("Secreto guardado — este dispositivo ya quedó identificado.");
      setSecreto("");
      cargarEstado();
    } catch (error) {
      toast.error(String(error));
    } finally {
      setGuardando(false);
    }
  }

  async function sincronizar() {
    setSincronizando(true);
    try {
      const resumen = await sincronizarConNube();
      setUltimoResumen(resumen);
      if (resumen.fallidos === 0) {
        toast.success(`Sincronizado — ${resumen.enviados} enviados.`);
      } else {
        toast.warning(`${resumen.enviados} enviados, ${resumen.fallidos} fallidos — reintenta más tarde.`);
      }
    } catch (error) {
      toast.error(String(error));
    } finally {
      setSincronizando(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <PantallaEncabezado titulo="Nube" />

      <div
        className="pantalla-cuerpo"
        style={{ minHeight: 0, flex: 1, display: "flex", flexDirection: "column", gap: "1.25rem", maxWidth: 480 }}
      >
        <section style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
          <h3 style={{ margin: 0 }}>Identidad del dispositivo</h3>

          {configurado === null && <p style={{ color: "var(--muted)" }}>Cargando…</p>}

          {configurado === true && (
            <p style={{ color: "var(--exito)" }}>
              Este dispositivo ya tiene su secreto configurado.
            </p>
          )}

          {configurado === false && (
            <>
              <p style={{ color: "var(--muted)", marginTop: 0 }}>
                Pegá acá el secreto que generó el panel de administración al dar de alta este
                dispositivo. Se guarda una sola vez.
              </p>
              <div style={{ display: "flex", gap: "0.5rem" }}>
                <input
                  type="password"
                  value={secreto}
                  onChange={(evento) => setSecreto(evento.target.value)}
                  placeholder="Secreto del dispositivo"
                  style={{ flex: 1 }}
                />
                <button
                  type="button"
                  className="boton boton-primario"
                  onClick={guardar}
                  disabled={guardando || !secreto.trim()}
                >
                  {guardando ? "Guardando…" : "Guardar"}
                </button>
              </div>
            </>
          )}
        </section>

        <section style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
          <h3 style={{ margin: 0 }}>Sincronización</h3>
          <div>
            <button
              type="button"
              className="boton"
              onClick={sincronizar}
              disabled={sincronizando || configurado !== true}
            >
              {sincronizando ? "Sincronizando…" : "Sincronizar ahora"}
            </button>
          </div>

          {ultimoResumen && (
            <p style={{ color: "var(--muted)", fontSize: "0.85rem" }}>
              Sitio {ultimoResumen.sitio_id} · dispositivo {ultimoResumen.dispositivo_id} (
              {ultimoResumen.tipo}) — {ultimoResumen.enviados} enviados, {ultimoResumen.fallidos}{" "}
              fallidos.
            </p>
          )}
        </section>
      </div>
    </div>
  );
}
