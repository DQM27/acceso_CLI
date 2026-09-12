/**
 * Convención para pantallas y componentes nuevos — mismo criterio que ya
 * sigue `comandos/mod.rs` del lado Tauri, ahora escrito acá:
 *
 * 1. Una pantalla nunca importa a otra pantalla. Si dos pantallas necesitan
 *    lo mismo, eso va a `componentes/` (ver `ListaFlotante.tsx`) o a un
 *    hook — nunca una pantalla llamando directo a otra. Única excepción
 *    real: cada pantalla con SU PROPIO Formulario* (Contratistas ↔
 *    FormularioContratista, etc.), que sigue siendo padre → hijo, no
 *    acoplamiento entre hermanas.
 * 2. `api/*.ts` es la única capa que llama `invoke()`. Ningún componente o
 *    pantalla invoca Tauri directo — el mapeo de tipos/errores del lado
 *    Rust queda en un solo lugar por dominio.
 * 3. Antes de escribir un `useState`/`useEffect` que "se parece a algo que
 *    ya vi" en otra pantalla, revisar `componentes/` primero. La
 *    duplicación que había entre `NuevoIngresoModal`/`SalidaModal`
 *    (buscador con lista flotante + navegación de flechas, casi idéntica
 *    en los dos) es justo el tipo de cosa que este punto evita — ya se
 *    sacó a `ListaFlotante.tsx`, no se vuelve a copiar.
 * 4. Un componente se separa a su propio archivo cuando mezcla markup con
 *    lógica que no le pertenece a la función que lo contiene — la señal
 *    que llevó a sacar `Sidebar.tsx` de `Shell` (que sí hace enrutamiento
 *    y orquesta modales, eso es responsabilidad suya).
 */
import { Suspense, lazy, startTransition, useEffect, useState, ViewTransition } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { listen } from "@tauri-apps/api/event";
import { Toaster, toast } from "sonner";
import {
  Building2,
  ClipboardList,
  History,
  IdCard,
  UserCheck,
  Users,
  UsersRound,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import Sidebar from "./componentes/Sidebar";
import MenuUsuario from "./componentes/MenuUsuario";
import SelectorTema from "./componentes/SelectorTema";
import BarraNube from "./componentes/BarraNube";
import type { EstadoConexionNube } from "./componentes/BarraNube";
import ErrorBoundary from "./componentes/ErrorBoundary";
import Login from "./pantallas/Login";
import PrimerArranque from "./pantallas/PrimerArranque";
import {
  buscarActualizacion,
  cerrarSesion,
  instalarActualizacion,
  requiereConfiguracionInicial,
  sincronizarConNube,
} from "./api";
import type { ResumenSincronizacion, UsuarioSesion } from "./api";
import { emitirActualizacion, iniciarRealtimeNube } from "./nubeRealtime";
import { SesionProvider } from "./contexto/SesionContexto";
import { BarraEstadoProvider, SeccionActivaProvider } from "./contexto/BarraEstadoContexto";

// Las pantallas y sus tablas se cargan al entrar a cada sección.
const Activos = lazy(() => import("./pantallas/Activos"));
const Visitas = lazy(() => import("./pantallas/Visitas"));
const Contratistas = lazy(() => import("./pantallas/Contratistas"));
const Empresas = lazy(() => import("./pantallas/Empresas"));
const Historial = lazy(() => import("./pantallas/Historial"));
const Auditoria = lazy(() => import("./pantallas/Auditoria"));
const Gafetes = lazy(() => import("./pantallas/Gafetes"));
const NuevoIngresoModal = lazy(() => import("./pantallas/NuevoIngresoModal"));
const SalidaModal = lazy(() => import("./pantallas/SalidaModal"));

type Pantalla =
  | { tipo: "cargando" }
  | { tipo: "requiere-configuracion-inicial" }
  | { tipo: "login" }
  | { tipo: "shell"; sesion: UsuarioSesion };

const CLAVE_SIDEBAR_COLAPSADO = "sidebar:colapsado";

/** `localStorage` puede fallar (modo privado, cuota llena) — mismo criterio
 * que `leerEstadoGuardado`/`guardarLayout` en `Tabla.tsx`: perder la
 * preferencia guardada no es motivo para romper nada, sólo se vuelve al
 * valor por defecto (expandido). */
function leerSidebarColapsado(): boolean {
  try {
    return localStorage.getItem(CLAVE_SIDEBAR_COLAPSADO) === "1";
  } catch {
    return false;
  }
}

function guardarSidebarColapsado(colapsado: boolean) {
  try {
    localStorage.setItem(CLAVE_SIDEBAR_COLAPSADO, colapsado ? "1" : "0");
  } catch {
    // Ver comentario de leerSidebarColapsado.
  }
}

export default function App() {
  const [pantalla, setPantalla] = useState<Pantalla>({ tipo: "cargando" });

  useEffect(() => {
    requiereConfiguracionInicial()
      .then((requiere) =>
        setPantalla(requiere ? { tipo: "requiere-configuracion-inicial" } : { tipo: "login" }),
      )
      .catch((error) => {
        // Deja intentar login igual — si el problema persiste, el propio
        // comando `login` lo va a reportar con su propio mensaje de error.
        console.error(error);
        setPantalla({ tipo: "login" });
      });
  }, []);

  if (pantalla.tipo === "cargando") {
    return null;
  }

  if (pantalla.tipo === "requiere-configuracion-inicial") {
    return <PrimerArranque onListo={() => setPantalla({ tipo: "login" })} />;
  }

  if (pantalla.tipo === "login") {
    return <Login onAutenticado={(sesion) => setPantalla({ tipo: "shell", sesion })} />;
  }

  return (
    <Shell
      sesion={pantalla.sesion}
      onCerrarSesion={() => {
        cerrarSesion().finally(() => setPantalla({ tipo: "login" }));
      }}
    />
  );
}

export type Seccion =
  | "activos"
  | "visitas"
  | "historial"
  | "contratistas"
  | "auditoria"
  | "empresas"
  | "gafetes";

/** Aplanado de autorización (ver docs/decisiones-tecnicas.md 2026-09-11):
 * ninguna sección se oculta por rol -- quien tiene una sesión válida puede
 * ver todo. Auditoría restringía a Root/Administrador (espejo de
 * `RolUsuario::puede(VerAuditoria)`, ya aplanado en
 * `src/domain/autorizacion.rs` para devolver `true` siempre), pero ese
 * gate era un `match` propio de esta pantalla que no pasaba por `puede()`
 * -- por eso no se detectó junto con el resto en esa misma pasada (mismo
 * tipo de gate duplicado que ya se encontró y cerró en la TUI).
 *
 * No hay una sección "Usuarios": administrar usuarios globales (alta,
 * edición, reset de contraseña de otro usuario) quedó exclusivo del panel
 * administrativo web (ver docs/planes-implementados/plan-autenticacion-supabase-auth.md) — el
 * escritorio ya no origina cambios contra esa tabla, salvo que la propia
 * sesión cambie su propia contraseña (`cambiarMiPassword`). */
const SECCIONES: {
  id: Seccion;
  etiqueta: string;
  Icono: LucideIcon;
}[] = [
  { id: "activos", etiqueta: "Activos", Icono: UserCheck },
  { id: "visitas", etiqueta: "Visitas", Icono: UsersRound },
  { id: "historial", etiqueta: "Historial", Icono: History },
  { id: "contratistas", etiqueta: "Contratistas", Icono: Users },
  { id: "auditoria", etiqueta: "Auditoría", Icono: ClipboardList },
  { id: "empresas", etiqueta: "Empresas", Icono: Building2 },
  { id: "gafetes", etiqueta: "Gafetes", Icono: IdCard },
];

/**
 * Interfaz central: sidebar izquierdo con las secciones + área de contenido
 * a la derecha. Cada sección nueva (ingresos, activos, historial...) sólo
 * agrega una entrada acá y su propio componente — no toca el resto.
 */
function Shell({
  sesion,
  onCerrarSesion,
}: {
  sesion: UsuarioSesion;
  onCerrarSesion: () => void;
}) {
  const [seccion, setSeccion] = useState<Seccion>("activos");
  // Cada sección visitada se queda MONTADA (oculta con CSS) en vez de
  // desmontarse al cambiar a otra -- antes, el `{seccion === "x" && <X/>}`
  // de abajo desmontaba la pantalla anterior por completo al elegir otra,
  // así que volver a una ya vista reconstruía todo de cero (nuevo fetch a
  // SQLite vía Tauri, AG Grid desde cero, scroll/filtro perdidos). Acá es
  // peor que en la web (donde al menos hay una descarga de red real de por
  // medio): todo vive en el mismo binario, nada que "descargar" -- no
  // había ninguna razón real para pagar ese costo cada vez. Sólo la
  // PRIMERA visita a cada sección monta su componente; volver después es
  // instantáneo.
  const [visitadas, setVisitadas] = useState<Seccion[]>(["activos"]);
  useEffect(() => {
    // `Promise.resolve().then(...)` en vez de llamar `setVisitadas` directo
    // -- ver el mismo comentario en Activos.tsx.
    Promise.resolve().then(() => {
      setVisitadas((actual) => (actual.includes(seccion) ? actual : [...actual, seccion]));
    });
  }, [seccion]);
  // React 19.3: `<ViewTransition>` sólo anima un cambio si ocurrió dentro de
  // una transición -- acá no hay router que la envuelva sola (a diferencia
  // de web-visitas con createBrowserRouter), así que el cambio de sección
  // se dispara a mano con startTransition. Las secciones ya están todas
  // montadas (ver comentario de arriba), el cambio real es sólo un toggle
  // de `display` -- React igual detecta la mutación dentro del contenedor
  // envuelto y dispara el cross-fade nativo del navegador.
  function cambiarSeccion(id: Seccion) {
    startTransition(() => setSeccion(id));
  }
  const [colapsado, setColapsado] = useState(leerSidebarColapsado);
  // La pantalla montada publica acá su propio texto (ver `useBarraEstado`) —
  // `null` mientras ninguna lo hizo todavía (primer render) o entre una
  // pantalla y la siguiente.
  const [mensajeEstado, setMensajeEstado] = useState<string | null>(null);

  function alternarColapsado() {
    setColapsado((actual) => {
      const siguiente = !actual;
      guardarSidebarColapsado(siguiente);
      return siguiente;
    });
  }

  const [modalNuevoIngreso, setModalNuevoIngreso] = useState(false);
  const [modalSalida, setModalSalida] = useState(false);
  // Sube en cada registro/salida/sincronización — Activos y Visitas lo usan
  // para refrescar su grilla/calendario aunque el cambio haya salido de
  // otra pantalla o de otro dispositivo (Realtime/pulso periódico).
  const [refrescarActivos, setRefrescarActivos] = useState(0);
  const [sincronizandoManual, setSincronizandoManual] = useState(false);
  // `null` hasta que `iniciarRealtimeNube` intenta conectar la primera vez.
  const [estadoConexionNube, setEstadoConexionNube] = useState<EstadoConexionNube>(null);

  // Ctrl+Shift+N/S (no Ctrl+N/S solos — esas convenciones quedan libres
  // para un "nuevo"/"salida" más genéricos más adelante) desde cualquier
  // pantalla: ambos modales son autosuficientes (buscan y registran sin
  // depender de qué sección esté abierta), así que no tiene sentido
  // atarlos a un botón dentro de Activos únicamente. Deshabilitados por
  // defecto mientras se escribe en un campo de texto (comportamiento por
  // defecto de la librería).
  useHotkeys("ctrl+shift+n", () => setModalNuevoIngreso(true), { preventDefault: true });
  useHotkeys("ctrl+shift+s", () => setModalSalida(true), { preventDefault: true });
  // Mismo atajo que la TUI clásica y --cli (Ctrl+Q cierra sesión desde
  // cualquier pantalla) — acá sin tarjeta de confirmación porque el botón
  // "Cerrar sesión" del sidebar tampoco la pide, así el atajo y el botón se
  // comportan igual.
  useHotkeys("ctrl+q", onCerrarSesion, { preventDefault: true });

  // Cualquier sincronización (Realtime, el pulso periódico, o "Sincronizar"
  // a mano) puede traer la baja/desactivación de quien tiene la sesión
  // abierta ACÁ MISMO -- el lado Rust ya cerró esa sesión
  // (`ejecutar_sincronizacion`, ver `ResumenSincronizacion::sesion_expulsada`),
  // esto sólo hace que la pantalla reaccione en vez de seguir mostrando el
  // Shell con una sesión que el backend ya no reconoce. `true` (en vez de
  // sumar a `refrescarActivos`) corta ahí: no tiene sentido refrescar
  // pantallas de una sesión que ya no existe.
  function manejarResumenSincronizacion(resumen: ResumenSincronizacion): boolean {
    if (resumen.sesion_expulsada) {
      toast.error("Tu usuario fue desactivado — se cerró la sesión.");
      onCerrarSesion();
      return true;
    }
    // `docs/pendientes.md`, "alertar luego al sincronizar": un ingreso que
    // se registró en este dispositivo mientras estaba offline y que la
    // nube dice que TAMBIÉN sigue activo en otro sitio ahora mismo. El
    // otro sitio ve esta misma alerta desde su propio lado -- cada
    // dispositivo revisa sus propios ingresos activos contra el mismo
    // estado remoto (ver `nube::contratistas_con_conflicto_activo`), sin
    // necesitar un canal de aviso aparte entre sitios. Sigue avisando en
    // cada sync mientras el conflicto no se resuelva (cerrando uno de los
    // dos ingresos) -- no es un error transitorio que convenga silenciar.
    for (const conflicto of resumen.conflictos_ingreso) {
      toast.warning(
        `${conflicto.contratista_nombre} tiene un ingreso activo acá Y en ${conflicto.sitio_conflicto} — hay que resolverlo.`,
      );
    }
    return false;
  }

  // Los avisos privados refrescan de inmediato; el pulso periódico recupera
  // cambios aunque se pierda el socket o el equipo haya estado sin conexión.
  // `setRefrescarActivos` dentro de `startTransition` (React 19.3): esto
  // puede llegar en cualquier momento (Realtime, pulso) mientras el usuario
  // está haciendo otra cosa -- sin la transición, el refetch/rerender que
  // dispara en cada sección montada compite por prioridad con lo que el
  // usuario esté tipeando/clickeando en ese instante.
  useEffect(() => {
    const cancelarRealtime = iniciarRealtimeNube({
      onSincronizado: (resumen) => {
        if (!manejarResumenSincronizacion(resumen)) {
          startTransition(() => setRefrescarActivos((n) => n + 1));
        }
      },
      onEstado: setEstadoConexionNube,
      usuario: { cedula: sesion.cedula, nombre: sesion.nombre },
    });
    const cancelarSincronizacionAutomatica = listen<ResumenSincronizacion>(
      "nube://sincronizado",
      ({ payload }) => {
        if (manejarResumenSincronizacion(payload)) return;
        startTransition(() => setRefrescarActivos((n) => n + 1));
        emitirActualizacion(payload);
      },
    );

    return () => {
      cancelarRealtime();
      cancelarSincronizacionAutomatica.then((cancelar) => cancelar());
    };
  }, [sesion.id]);

  // Botón "Sincronizar" de la barra de estado (`BarraNube.tsx`) — visible
  // para cualquier rol activo, ver su doc-comment. `sincronizar_con_nube`
  // ya falla con un mensaje claro (`GestionNubeError::SinSecreto`) si este
  // dispositivo todavía no tiene el secreto configurado, así que no hace
  // falta ocultar el botón para quien no puede configurarlo (eso sigue
  // siendo exclusivo de ROOT en la pantalla Nube).
  async function sincronizarManualmente() {
    setSincronizandoManual(true);
    try {
      const resumen = await sincronizarConNube();
      if (manejarResumenSincronizacion(resumen)) return;
      startTransition(() => setRefrescarActivos((n) => n + 1));
      emitirActualizacion(resumen, "manual");
      if (resumen.fallidos === 0) {
        toast.success(`Sincronizado — ${resumen.enviados} enviados.`);
      } else {
        toast.warning(
          `${resumen.enviados} enviados, ${resumen.fallidos} fallidos — reintenta más tarde.`,
        );
      }
    } catch (error) {
      toast.error(String(error));
    } finally {
      setSincronizandoManual(false);
    }
  }

  // Una sola vez por sesión (no cada X minutos todavía — la app se abre y
  // cierra bastante seguido, esto ya cubre el caso normal). Falla en
  // silencio a propósito: sin conexión o GitHub caído no debe interrumpir a
  // alguien que ya está trabajando, sólo no hay novedad que avisar.
  useEffect(() => {
    let vigente = true;
    buscarActualizacion()
      .then((actualizacion) => {
        if (!vigente || !actualizacion) return;
        toast(`Versión ${actualizacion.version} disponible`, {
          description: "Se descarga, se instala y la app se reinicia sola.",
          duration: Infinity,
          action: {
            label: "Actualizar",
            onClick: () => {
              toast.promise(instalarActualizacion(actualizacion), {
                loading: "Descargando actualización…",
                success: "Actualizado — reiniciando…",
                error: (error) => `No se pudo actualizar: ${String(error)}`,
              });
            },
          },
        });
      })
      .catch((error) => console.error("No se pudo buscar actualizaciones:", error));
    return () => {
      vigente = false;
    };
  }, []);

  return (
    <SesionProvider value={sesion.id}>
      <BarraEstadoProvider value={setMensajeEstado}>
        <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
          <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
            <Sidebar
              secciones={SECCIONES}
              seccionActual={seccion}
              onCambiarSeccion={cambiarSeccion}
              colapsado={colapsado}
              onToggleColapsado={alternarColapsado}
            />

            <main style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column" }}>
              {/* Ver cambiarSeccion arriba -- las secciones ya están todas
                  montadas, el cross-fade es sobre el toggle de `display`
                  dentro de este contenedor, no sobre mount/unmount. */}
              <ViewTransition>
              {visitadas.map((id) => (
                <div
                  key={id}
                  style={{
                    display: id === seccion ? "flex" : "none",
                    flexDirection: "column",
                    flex: 1,
                    minHeight: 0,
                  }}
                >
                  {/* Un ErrorBoundary por sección (ya no `key={seccion}`
                      compartido) -- con las secciones montadas para
                      siempre, resetear por key ya no aplica igual: si una
                      rompe, queda rota hasta reiniciar la app (el propio
                      mensaje ya lo decía como salida). Lo que se pierde es
                      "reelegir la misma sección para reintentar" -- costo
                      aceptado, es un caso raro y la app entera sigue
                      funcionando en todas las demás secciones. */}
                  <ErrorBoundary mensaje="Esta sección no pudo cargar. La sesión sigue activa — elegí otra desde el menú, o reiniciá la app si el problema persiste.">
                    {/* Fallback `null`: las pantallas lazy vienen del mismo
                        bundle local (nada de red de por medio), el chunk
                        carga en milisegundos — no vale la pena un spinner
                        que sólo parpadearía. Suspense por sección, no uno
                        compartido, para que la primera visita a una
                        sección nueva no tape las que ya están montadas. */}
                    <Suspense fallback={null}>
                      {/* Ver el doc-comment de SeccionActivaContexto --
                          sin esto, useBarraEstado no se entera de cuándo
                          esta sección deja de ser la visible (ya no se
                          desmonta) y el mensaje de una sección vieja
                          queda pegado en la barra de estado. */}
                      <SeccionActivaProvider value={id === seccion}>
                        {id === "activos" ? (
                          <Activos
                            refrescarSenal={refrescarActivos}
                            onAbrirNuevoIngreso={() => setModalNuevoIngreso(true)}
                            onAbrirSalida={() => setModalSalida(true)}
                          />
                        ) : id === "visitas" ? (
                          <Visitas refrescarSenal={refrescarActivos} />
                        ) : id === "historial" ? (
                          <Historial />
                        ) : id === "contratistas" ? (
                          <Contratistas actorRol={sesion.rol} />
                        ) : id === "auditoria" ? (
                          <Auditoria />
                        ) : id === "empresas" ? (
                          <Empresas />
                        ) : (
                          <Gafetes />
                        )}
                      </SeccionActivaProvider>
                    </Suspense>
                  </ErrorBoundary>
                </div>
              ))}
              </ViewTransition>
            </main>
          </div>

          {/* De lado a lado, debajo de sidebar + contenido — mismo lugar que
              la barra de estado de VSC. Cada pantalla publica su propio texto
              acá (`useBarraEstado`) en vez de dibujarlo ella misma sobre la
              grilla; a la derecha, el usuario (antes fijo en el sidebar). */}
          <div className="barra-estado">
            <span>{mensajeEstado}</span>
            <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
              <BarraNube
                sincronizando={sincronizandoManual}
                onSincronizar={sincronizarManualmente}
                estadoConexion={estadoConexionNube}
              />
              <SelectorTema />
              <MenuUsuario sesion={sesion} onCerrarSesion={onCerrarSesion} />
            </div>
          </div>

          <Suspense fallback={null}>
            {modalNuevoIngreso && (
              <NuevoIngresoModal
                onRegistrado={() => setRefrescarActivos((n) => n + 1)}
                onCerrar={() => setModalNuevoIngreso(false)}
              />
            )}

            {modalSalida && (
              <SalidaModal
                onRegistrado={() => setRefrescarActivos((n) => n + 1)}
                onCerrar={() => setModalSalida(false)}
              />
            )}
          </Suspense>

          {/* theme="system": mismo criterio que el resto de la app (paleta
              clara/oscura sigue `prefers-color-scheme`, sin toggle manual
              todavía) — estilizado con las variables propias en index.css, no
              los colores por defecto de sonner. */}
          <Toaster theme="system" position="bottom-right" richColors={false} />
        </div>
      </BarraEstadoProvider>
    </SesionProvider>
  );
}
