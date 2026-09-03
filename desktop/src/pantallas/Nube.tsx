import { useEffect, useState } from "react";
import { toast } from "sonner";
import { listen } from "@tauri-apps/api/event";
import { EVENTO_NUBE_ACTUALIZADA } from "../nubeRealtime";
import type { NubeActualizadaDetalle } from "../nubeRealtime";
import {
  cerrarIngresoRemoto,
  fallosPermanentesNube,
  guardarSecretoDispositivo,
  listarIngresosRemotos,
  secretoDispositivoGuardado,
} from "../api";
import type { IngresoRemoto, ResumenSincronizacion } from "../api";
import { textoHora } from "../tiempo";

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
  const [ultimoResumen, setUltimoResumen] = useState<ResumenSincronizacion | null>(null);
  const [remotos, setRemotos] = useState<IngresoRemoto[]>([]);
  const [cerrandoUuid, setCerrandoUuid] = useState<string | null>(null);
  const [fallosPermanentes, setFallosPermanentes] = useState(0);

  function cargarEstado() {
    secretoDispositivoGuardado()
      .then(setConfigurado)
      .catch((error) => toast.error(String(error)));
  }

  function cargarRemotos() {
    listarIngresosRemotos()
      .then(setRemotos)
      .catch((error) => toast.error(String(error)));
  }

  function cargarFallosPermanentes() {
    fallosPermanentesNube()
      .then(setFallosPermanentes)
      .catch((error) => toast.error(String(error)));
  }

  useEffect(cargarEstado, []);
  useEffect(cargarRemotos, []);
  useEffect(cargarFallosPermanentes, []);

  // El disparador automático (`crate::iniciar_sincronizacion_automatica`,
  // cada 2 minutos mientras la app está abierta) corre en segundo plano sin
  // que nadie apriete el botón -- este listener es sólo para que, si esta
  // pantalla está abierta cuando eso pasa, se vea el resultado sin recargar.
  useEffect(() => {
    const cancelar = listen<ResumenSincronizacion>("nube://sincronizado", (evento) => {
      setUltimoResumen(evento.payload);
      cargarRemotos();
      cargarFallosPermanentes();
    });
    return () => {
      cancelar.then((f) => f());
    };
  }, []);

  useEffect(() => {
    function alActualizar(evento: Event) {
      const detalle = (evento as CustomEvent<NubeActualizadaDetalle>).detail;
      setUltimoResumen(detalle.resumen);
      cargarRemotos();
      cargarFallosPermanentes();
    }

    window.addEventListener(EVENTO_NUBE_ACTUALIZADA, alActualizar);
    return () => window.removeEventListener(EVENTO_NUBE_ACTUALIZADA, alActualizar);
  }, []);

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

  async function cerrar(uuid: string) {
    setCerrandoUuid(uuid);
    try {
      await cerrarIngresoRemoto(uuid);
      toast.success("Ingreso cerrado.");
      setRemotos((actuales) => actuales.filter((remoto) => remoto.uuid !== uuid));
    } catch (error) {
      toast.error(String(error));
    } finally {
      setCerrandoUuid(null);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div
        className="pantalla-cuerpo"
        style={{ minHeight: 0, flex: 1, display: "flex", flexDirection: "column", gap: "1.25rem", maxWidth: 560 }}
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
          <p style={{ color: "var(--muted)", marginTop: 0, fontSize: "0.85rem" }}>
            El botón "Sincronizar" vive en la barra de estado (abajo a la derecha), disponible
            desde cualquier pantalla.
          </p>

          {ultimoResumen && (
            <p style={{ color: "var(--muted)", fontSize: "0.85rem" }}>
              Sitio {ultimoResumen.sitio_id} · dispositivo {ultimoResumen.dispositivo_id} (
              {ultimoResumen.tipo}) — {ultimoResumen.enviados} enviados, {ultimoResumen.fallidos}{" "}
              fallidos, {ultimoResumen.cierres_recibidos} cierres recibidos.
            </p>
          )}

          {fallosPermanentes > 0 && (
            <p style={{ color: "var(--error)", fontSize: "0.85rem" }}>
              {fallosPermanentes} {fallosPermanentes === 1 ? "elemento" : "elementos"} dejaron de
              reintentarse solos tras agotar los intentos automáticos — necesita revisión manual.
            </p>
          )}
        </section>

        <section style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
          <h3 style={{ margin: 0 }}>Abiertos en el otro dispositivo del sitio</h3>
          <p style={{ color: "var(--muted)", marginTop: 0, fontSize: "0.85rem" }}>
            Se actualiza con la sincronización y con avisos en vivo.
          </p>

          {remotos.length === 0 && (
            <p style={{ color: "var(--muted)" }}>Nada abierto del otro lado por ahora.</p>
          )}

          {remotos.length > 0 && (
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.85rem" }}>
              <thead>
                <tr>
                  <th style={{ textAlign: "left", padding: "0.3rem 0.5rem" }}>Contratista</th>
                  <th style={{ textAlign: "left", padding: "0.3rem 0.5rem" }}>Entrada</th>
                  <th style={{ textAlign: "left", padding: "0.3rem 0.5rem" }}>Registrado por</th>
                  <th style={{ padding: "0.3rem 0.5rem" }} />
                </tr>
              </thead>
              <tbody>
                {remotos.map((remoto) => (
                  <tr key={remoto.uuid}>
                    <td style={{ padding: "0.3rem 0.5rem" }}>{remoto.contratista_nombre}</td>
                    <td style={{ padding: "0.3rem 0.5rem" }}>{textoHora(remoto.hora_entrada)}</td>
                    <td style={{ padding: "0.3rem 0.5rem" }}>{remoto.usuario_entrada_nombre ?? "—"}</td>
                    <td style={{ padding: "0.3rem 0.5rem" }}>
                      <button
                        type="button"
                        className="boton"
                        onClick={() => cerrar(remoto.uuid)}
                        disabled={cerrandoUuid === remoto.uuid}
                      >
                        {cerrandoUuid === remoto.uuid ? "Cerrando…" : "Registrar salida"}
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </section>
      </div>
    </div>
  );
}
