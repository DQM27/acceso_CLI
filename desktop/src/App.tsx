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
import {
  Suspense,
  lazy,
  startTransition,
  useEffect,
  useMemo,
  useRef,
  useState,
  ViewTransition,
} from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { Toaster, toast } from "sonner";
import {
  BadgeCheck,
  Boxes,
  Building2,
  ClipboardList,
  DoorOpen,
  Download,
  HardHat,
  History,
  IdCard,
  Loader2,
  LogOut,
  Route,
  Truck,
  UserCheck,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import marca from "./assets/marca.png";
import Sidebar from "./componentes/Sidebar";
import MenuUsuario from "./componentes/MenuUsuario";
import SelectorTema from "./componentes/SelectorTema";
import { guardarPreferencia, leerPreferencia } from "./preferencias";
import BarraNube from "./componentes/BarraNube";
import type { EstadoConexionNube } from "./componentes/BarraNube";
import ErrorBoundary from "./componentes/ErrorBoundary";
import Login from "./pantallas/Login";
import PrimerArranque from "./pantallas/PrimerArranque";
import {
  buscarActualizacion,
  cerrarSesion,
  instalarActualizacion,
  mostrarVentanaPrincipal,
  requiereConfiguracionInicial,
  sincronizarConNube,
} from "./api";
import type { ResumenSincronizacion, Update, UsuarioSesion } from "./api";
import { emitirActualizacion, iniciarRealtimeNube } from "./nubeRealtime";
import { textoHora } from "./tiempo";
import { SesionProvider } from "./contexto/SesionContexto";
import {
  AccionBarraEstadoProvider,
  BarraEstadoProvider,
  SeccionActivaProvider,
} from "./contexto/BarraEstadoContexto";
import type { AccionBarraEstado } from "./contexto/BarraEstadoContexto";

/** Piso de cuánto se ve el splash (`splashscreen.html`), aunque la pantalla
 * real esté lista antes -- sin esto, en un arranque rápido el splash pasaba
 * tan fugaz que ni se alcanzaba a leer (reportado 2026-09-19). No compite
 * con `ESPERA_MAXIMA_SPLASH` de `lib.rs` (8s, la red de seguridad si el
 * frontend nunca avisa) -- éste es un PISO sobre el camino normal, aquél un
 * TECHO sobre el camino de emergencia; entre uno y otro el splash dura entre
 * `DURACION_MINIMA_SPLASH_MS` y `ESPERA_MAXIMA_SPLASH` según qué tan rápido
 * esté todo. */
const DURACION_MINIMA_SPLASH_MS = 2000;

/** Instante de referencia para medir cuánto lleva visible el splash (ver
 * `DURACION_MINIMA_SPLASH_MS`) -- módulo, no dentro de `App`, porque tiene
 * que capturarse una sola vez, apenas este archivo se evalúa (lo más cerca
 * posible del instante real en que Tauri mostró la ventana del splash), no
 * en cada montaje/remontaje del componente. */
const inicioSplash = performance.now();

// Las pantallas y sus tablas se cargan al entrar a cada sección.
const Activos = lazy(() => import("./pantallas/Activos"));
const Visitas = lazy(() => import("./pantallas/Visitas"));
const Contratistas = lazy(() => import("./pantallas/Contratistas"));
const Empresas = lazy(() => import("./pantallas/Empresas"));
const Historial = lazy(() => import("./pantallas/Historial"));
const Auditoria = lazy(() => import("./pantallas/Auditoria"));
const Gafetes = lazy(() => import("./pantallas/Gafetes"));
const Rutas = lazy(() => import("./pantallas/Rutas"));
const CatalogoRutas = lazy(() => import("./pantallas/CatalogoRutas"));
const Proveedores = lazy(() => import("./pantallas/Proveedores"));
const GafetesProvisionales = lazy(() => import("./pantallas/GafetesProvisionales"));
const NuevoIngresoModal = lazy(() => import("./pantallas/NuevoIngresoModal"));
const SalidaModal = lazy(() => import("./pantallas/SalidaModal"));

type Pantalla =
  | { tipo: "cargando" }
  | { tipo: "requiere-configuracion-inicial" }
  | { tipo: "login" }
  | { tipo: "shell"; sesion: UsuarioSesion };

// Estado del menú lateral, por usuario (ver `preferencias.ts`): cada quien
// tiene su propio orden, secciones ocultas y colapsado en la misma PC.
const CLAVE_SIDEBAR_COLAPSADO = "sidebar:colapsado";

/** Sin guardado (o si `localStorage` falla): expandido. */
function leerSidebarColapsado(usuarioId: number): boolean {
  return leerPreferencia(CLAVE_SIDEBAR_COLAPSADO, usuarioId) === "1";
}

function guardarSidebarColapsado(usuarioId: number, colapsado: boolean) {
  guardarPreferencia(CLAVE_SIDEBAR_COLAPSADO, usuarioId, colapsado ? "1" : "0");
}

const CLAVE_SIDEBAR_ORDEN = "sidebar:orden";
const CLAVE_SIDEBAR_OCULTAS = "sidebar:ocultas";

/** Filtra ids que ya no existen en `SECCIONES` -- si una sección se quita
 * del código, el `localStorage` de una instalación vieja no debe romper el
 * sidebar ni resucitarla como "oculta"/"reordenada" fantasma. */
function seccionValida(id: string): id is Seccion {
  return SECCIONES.some((seccion) => seccion.id === id);
}

/** Lista de secciones guardada; `null` si no hay nada o el JSON está roto. */
function leerListaSecciones(clave: string, usuarioId: number): Seccion[] | null {
  const guardado = leerPreferencia(clave, usuarioId);
  if (!guardado) return null;
  try {
    const ids = JSON.parse(guardado) as unknown[];
    return ids.filter((id): id is Seccion => typeof id === "string" && seccionValida(id));
  } catch {
    return null;
  }
}

/** Sin guardado todavía (preferencia nunca tocada): el orden por defecto es
 * el literal de `SECCIONES`. */
function leerSidebarOrden(usuarioId: number): Seccion[] {
  return (
    leerListaSecciones(CLAVE_SIDEBAR_ORDEN, usuarioId) ?? SECCIONES.map((seccion) => seccion.id)
  );
}

function guardarSidebarOrden(usuarioId: number, orden: Seccion[]) {
  guardarPreferencia(CLAVE_SIDEBAR_ORDEN, usuarioId, JSON.stringify(orden));
}

function leerSidebarOcultas(usuarioId: number): Seccion[] {
  return leerListaSecciones(CLAVE_SIDEBAR_OCULTAS, usuarioId) ?? [];
}

function guardarSidebarOcultas(usuarioId: number, ocultas: Seccion[]) {
  guardarPreferencia(CLAVE_SIDEBAR_OCULTAS, usuarioId, JSON.stringify(ocultas));
}

export default function App() {
  const [pantalla, setPantalla] = useState<Pantalla>({ tipo: "cargando" });
  // Evita reinvocar `mostrarVentanaPrincipal` en transiciones posteriores
  // (login → shell al autenticar, shell → login al cerrar sesión, etc.) --
  // sólo hace falta la primera vez que hay algo real que mostrar. Puesta en
  // `true` recién dentro del efecto de abajo, no acá: bajo `StrictMode`
  // (activo en dev, ver `main.tsx`) React monta/desmonta cada efecto una vez
  // de más, y si este flag se marcara antes del segundo
  // `requestAnimationFrame` (en vez de recién cuando ese frame corre de
  // verdad), el primer montaje "de prueba" lo dejaría marcado y el segundo
  // montaje real nunca llegaría a invocar nada.
  const yaMostroVentanaPrincipal = useRef(false);

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

  useEffect(() => {
    // Recién acá (no en el `.then()/.finally()` de arriba) es seguro avisarle
    // a Rust que muestre la ventana principal: este efecto corre DESPUÉS de
    // que React ya commiteó el render de la pantalla real (login o alta
    // inicial), mientras que el `.finally()` de la promesa se dispara ANTES
    // de ese commit -- llamar `mostrarVentanaPrincipal` ahí mostraba la
    // ventana un instante antes de que el DOM tuviera ese contenido (un
    // parpadeo en blanco), que es lo que reportado 2026-09-18 llevó a
    // sacarlo por completo y dejar sólo el timer fijo del lado Rust como
    // único mecanismo -- ver `configurar_cierre_de_splash` en lib.rs para el
    // reemplazo de ese timer por una red de seguridad de verdad.
    //
    // El doble `requestAnimationFrame` espera un ciclo de pintado COMPLETO
    // del browser (no sólo el commit de React, que no garantiza que ya se
    // pintó en pantalla) antes de invocar: el primer rAF corre justo antes
    // de pintar el frame donde se aplicó el nuevo estado, el segundo ya
    // corre después de ese pintado. Encima de eso, `DURACION_MINIMA_SPLASH_MS`
    // -- ver su comentario -- así que en un arranque rápido esto puede
    // terminar esperando el resto de ese piso en vez de invocar apenas pinta.
    if (pantalla.tipo === "cargando" || yaMostroVentanaPrincipal.current) return;

    let cancelado = false;
    let cancelarSegundoFrame = () => {};
    let cancelarEsperaMinima = () => {};
    const idPrimerFrame = requestAnimationFrame(() => {
      const idSegundoFrame = requestAnimationFrame(() => {
        if (cancelado) return;
        const faltante = DURACION_MINIMA_SPLASH_MS - (performance.now() - inicioSplash);
        const idTimeout = window.setTimeout(
          () => {
            if (cancelado) return;
            yaMostroVentanaPrincipal.current = true;
            mostrarVentanaPrincipal().catch(console.error);
          },
          Math.max(0, faltante),
        );
        cancelarEsperaMinima = () => window.clearTimeout(idTimeout);
      });
      cancelarSegundoFrame = () => cancelAnimationFrame(idSegundoFrame);
    });
    return () => {
      cancelado = true;
      cancelAnimationFrame(idPrimerFrame);
      cancelarSegundoFrame();
      cancelarEsperaMinima();
    };
  }, [pantalla.tipo]);

  if (pantalla.tipo === "cargando") {
    // Reemplaza el `return null` de antes -- mientras Rust abre la base y
    // corre migraciones (puede tardar varios segundos, sobre todo en el
    // primer arranque o si hubo que recuperar una base dañada), la ventana
    // quedaba con el interior en blanco hasta que esto resolvía (reportado
    // 2026-09-18). Mismo fondo/tarjeta que Login para que no haya un salto
    // visual raro al pasar de esta pantalla a la siguiente.
    return (
      <div className="grid min-h-full place-items-center bg-fondo px-6 py-10 text-texto">
        <div className="flex flex-col items-center gap-4">
          <div className="marca-sello" aria-hidden="true">
            <img src={marca} alt="" />
          </div>
          <Loader2 className="size-6 animate-spin text-muted" aria-hidden="true" />
        </div>
      </div>
    );
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
  | "rutas"
  | "historial"
  | "contratistas"
  | "auditoria"
  | "empresas"
  | "gafetes"
  | "catalogoRutas"
  | "proveedores"
  | "gafetesProvisionales";

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
const TODAS_LAS_SECCIONES: {
  id: Seccion;
  etiqueta: string;
  Icono: LucideIcon;
}[] = [
  { id: "activos", etiqueta: "Activos", Icono: UserCheck },
  // Antes UsersRound -- se veía casi igual que el ícono de Contratistas
  // (Users, ambos "grupo de personas") a este tamaño; DoorOpen distingue
  // de un vistazo (hallazgo real del usuario, 2026-09-21).
  { id: "visitas", etiqueta: "Visitas", Icono: DoorOpen },
  { id: "rutas", etiqueta: "Rutas", Icono: Route },
  { id: "historial", etiqueta: "Historial", Icono: History },
  // Antes Users -- casco de construcción es mas tematico para
  // "contratistas" y de paso deja de parecerse al icono de Visitas
  // (pedido explicito del usuario 2026-09-21).
  { id: "contratistas", etiqueta: "Contratistas", Icono: HardHat },
  { id: "auditoria", etiqueta: "Auditoría", Icono: ClipboardList },
  { id: "empresas", etiqueta: "Empresas", Icono: Building2 },
  { id: "gafetes", etiqueta: "Gafetes", Icono: IdCard },
  { id: "catalogoRutas", etiqueta: "Catálogo KOF", Icono: Truck },
  { id: "proveedores", etiqueta: "Proveedores", Icono: Boxes },
  { id: "gafetesProvisionales", etiqueta: "KOF", Icono: BadgeCheck },
];

/** Secciones sin terminar, ocultas de la interfaz (pedido del usuario
 * 2026-09-23: que no se vean a medias en una demostración). El código de
 * cada pantalla sigue intacto -- para volver a mostrar una, basta con
 * sacarla de acá. "Catálogo KOF" también: los encargados que usa "Gafetes
 * KOF" se administran por SQL directo en Supabase (decisión del usuario) y
 * llegan igual por `recibir_catalogo_rutas_del_sitio`. */
const SECCIONES_EN_DESARROLLO: ReadonlySet<Seccion> = new Set<Seccion>([
  "visitas",
  "rutas",
  "catalogoRutas",
]);

const SECCIONES = TODAS_LAS_SECCIONES.filter(
  (seccion) => !SECCIONES_EN_DESARROLLO.has(seccion.id),
);

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
  const [colapsado, setColapsado] = useState(() => leerSidebarColapsado(sesion.id));
  // La pantalla montada publica acá su propio texto (ver `useBarraEstado`) —
  // `null` mientras ninguna lo hizo todavía (primer render) o entre una
  // pantalla y la siguiente.
  const [mensajeEstado, setMensajeEstado] = useState<string | null>(null);
  // Botón de acción de la pantalla activa junto al mensaje (ver
  // `useAccionBarraEstado`) -- ej. "Registrar salida (2)" en Activos.
  const [accionEstado, setAccionEstado] = useState<AccionBarraEstado | null>(null);

  function alternarColapsado() {
    setColapsado((actual) => {
      const siguiente = !actual;
      guardarSidebarColapsado(sesion.id, siguiente);
      return siguiente;
    });
  }

  // Sidebar "inteligente" (mostrar/ocultar + reordenar, mismo espíritu que
  // la barra de actividad de VS Code) -- `orden` guarda TODOS los ids
  // (incluidos los ocultos, para poder volver a mostrarlos desde el menú
  // contextual del sidebar), `ocultas` es el subconjunto no visible. Ambos
  // persisten aparte de `colapsado` -- son ejes independientes (una sección
  // puede estar oculta sin importar si el sidebar está colapsado o no).
  const [ordenSidebar, setOrdenSidebar] = useState(() => leerSidebarOrden(sesion.id));
  const [seccionesOcultas, setSeccionesOcultas] = useState(() => leerSidebarOcultas(sesion.id));

  const seccionesOrdenadas = useMemo(() => {
    const porId = new Map(SECCIONES.map((seccion) => [seccion.id, seccion]));
    const ordenadas = ordenSidebar
      .map((id) => porId.get(id))
      .filter((seccion): seccion is (typeof SECCIONES)[number] => seccion !== undefined);
    // Cubre secciones nuevas agregadas al código después de que esta
    // instalación ya guardó un orden -- aparecen al final en vez de
    // desaparecer del sidebar.
    const faltantes = SECCIONES.filter((seccion) => !ordenSidebar.includes(seccion.id));
    return [...ordenadas, ...faltantes];
  }, [ordenSidebar]);

  const seccionesVisibles = useMemo(
    () => seccionesOrdenadas.filter((seccion) => !seccionesOcultas.includes(seccion.id)),
    [seccionesOrdenadas, seccionesOcultas],
  );

  // Si la sección activa se ocultó, hay que salir de ella -- de lo
  // contrario el usuario queda viendo una pantalla que ya no tiene entrada
  // en el sidebar para volver a elegir ni forma de saber dónde está parado.
  useEffect(() => {
    if (seccionesVisibles.length > 0 && !seccionesVisibles.some((s) => s.id === seccion)) {
      cambiarSeccion(seccionesVisibles[0].id);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [seccionesVisibles]);

  function reordenarSidebar(orden: Seccion[]) {
    setOrdenSidebar(orden);
    guardarSidebarOrden(sesion.id, orden);
  }

  function alternarVisibilidadSeccion(id: Seccion, visible: boolean) {
    setSeccionesOcultas((actual) => {
      const siguiente = visible ? actual.filter((x) => x !== id) : [...actual, id];
      // Nunca ocultar la última sección visible -- dejaría el sidebar
      // vacío y sin forma de deshacerlo desde la UI.
      if (siguiente.length >= SECCIONES.length) return actual;
      guardarSidebarOcultas(sesion.id, siguiente);
      return siguiente;
    });
  }

  function restablecerSidebar() {
    const ordenPorDefecto = SECCIONES.map((seccion) => seccion.id);
    setOrdenSidebar(ordenPorDefecto);
    setSeccionesOcultas([]);
    guardarSidebarOrden(sesion.id, ordenPorDefecto);
    guardarSidebarOcultas(sesion.id, []);
  }

  const [modalNuevoIngreso, setModalNuevoIngreso] = useState(false);
  const [modalSalida, setModalSalida] = useState(false);
  // Sube en cada registro/salida/sincronización — Activos y Visitas lo usan
  // para refrescar su grilla/calendario aunque el cambio haya salido de
  // otra pantalla o de otro dispositivo (Realtime/pulso periódico).
  const [refrescarActivos, setRefrescarActivos] = useState(0);
  const [sincronizandoManual, setSincronizandoManual] = useState(false);
  const [buscandoActualizacion, setBuscandoActualizacion] = useState(false);
  // `null` hasta que `iniciarRealtimeNube` intenta conectar la primera vez.
  const [estadoConexionNube, setEstadoConexionNube] = useState<EstadoConexionNube>(null);

  // Ctrl+N/S globales, desde cualquier pantalla: ambos modales son
  // autosuficientes (buscan y registran sin depender de qué sección esté
  // abierta), así que no tiene sentido atarlos a un botón dentro de Activos
  // únicamente. Antes eran Ctrl+Shift+N/S (para dejar Ctrl+N/S libres para
  // un "crear/nuevo" local de cada pantalla) -- simplificado a un solo par
  // de atajos en toda la app (pedido explícito del usuario, 2026-09-21): los
  // Ctrl+N locales de Contratistas/Empresas/Gafetes se quitaron para no
  // competir con este. Deshabilitados por defecto mientras se escribe en un
  // campo de texto (comportamiento por defecto de la librería).
  useHotkeys("ctrl+n", () => setModalNuevoIngreso(true), { preventDefault: true });
  useHotkeys("ctrl+s", () => setModalSalida(true), { preventDefault: true });
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
    // Mismo criterio que `conflictos_ingreso`, pero para visitas (ver
    // `nube::visitantes_con_conflicto_activo`).
    for (const conflicto of resumen.conflictos_movimiento_visita) {
      toast.warning(
        `${conflicto.visitante_nombre} tiene una visita activa acá Y en ${conflicto.sitio_conflicto} — hay que resolverlo.`,
      );
    }
    // Mismo criterio que `conflictos_ingreso`, pero para proveedores (ver
    // `nube::proveedores_con_conflicto_activo`).
    for (const conflicto of resumen.conflictos_ingreso_proveedor) {
      toast.warning(
        `${conflicto.nombre} tiene un ingreso de proveedor activo acá Y en ${conflicto.sitio_conflicto} — hay que resolverlo.`,
      );
    }
    // A diferencia de los tres de arriba (simétricos: ambos lados "tienen
    // razón" hasta que alguien decide), acá Postgres ya decidió -- el
    // ingreso local de ESTE dispositivo es el que no quedó válido en la
    // nube, así que el aviso lo dice con esa certeza. Sólo informativo por
    // ahora (fase 3, PR #62): sin botón de acción directa -- se deja para
    // una vuelta aparte si hace falta, una vez visto el comportamiento
    // real.
    for (const conflicto of resumen.conflictos_gafete) {
      toast.warning(
        `El ingreso de ${conflicto.contratista_nombre} con gafete ${conflicto.gafete_numero} (${textoHora(conflicto.fecha_hora_ingreso)}) no quedó registrado en la nube — otro dispositivo de este sitio ya lo tiene asignado.`,
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

  function ofrecerActualizacion(actualizacion: Update) {
    toast(`Versión ${actualizacion.version} disponible`, {
      id: "actualizacion-disponible",
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
  }

  // Una vez al abrir (la app se abre y cierra bastante seguido, esto cubre
  // el caso normal). Falla en silencio a propósito: sin conexión o GitHub
  // caído no debe interrumpir a alguien que ya está trabajando, sólo no hay
  // novedad que avisar. Para buscar a mano está el botón de la barra de
  // estado (`buscarActualizacionManual`), que sí avisa siempre.
  useEffect(() => {
    let vigente = true;
    buscarActualizacion()
      .then((actualizacion) => {
        if (vigente && actualizacion) ofrecerActualizacion(actualizacion);
      })
      .catch((error) => console.error("No se pudo buscar actualizaciones:", error));
    return () => {
      vigente = false;
    };
  }, []);

  // Botón "Buscar actualización" de la barra de estado — pedido del usuario
  // 2026-09-23: la búsqueda automática sólo corre al abrir, y con la app
  // abierta todo el turno una versión nueva no se enteraba nadie. A
  // diferencia de la automática, acá sí se avisa el resultado (también
  // "ya está al día" y los errores): quien lo pulsó espera una respuesta.
  async function buscarActualizacionManual() {
    setBuscandoActualizacion(true);
    try {
      const actualizacion = await buscarActualizacion();
      if (actualizacion) {
        ofrecerActualizacion(actualizacion);
      } else {
        const version = await getVersion().catch(() => null);
        toast.success(
          version ? `Ya tienes la última versión (${version}).` : "Ya tienes la última versión.",
        );
      }
    } catch (error) {
      toast.error(`No se pudo buscar actualizaciones: ${String(error)}`);
    } finally {
      setBuscandoActualizacion(false);
    }
  }

  return (
    <SesionProvider value={sesion.id}>
      <BarraEstadoProvider value={setMensajeEstado}>
      <AccionBarraEstadoProvider value={setAccionEstado}>
        <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
          <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
            <Sidebar
              secciones={seccionesOrdenadas}
              ocultas={seccionesOcultas}
              seccionActual={seccion}
              onCambiarSeccion={cambiarSeccion}
              colapsado={colapsado}
              onToggleColapsado={alternarColapsado}
              onReordenar={reordenarSidebar}
              onCambiarVisibilidad={alternarVisibilidadSeccion}
              onRestablecer={restablecerSidebar}
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
                        ) : id === "rutas" ? (
                          <Rutas refrescarSenal={refrescarActivos} />
                        ) : id === "historial" ? (
                          <Historial />
                        ) : id === "contratistas" ? (
                          <Contratistas actorRol={sesion.rol} />
                        ) : id === "auditoria" ? (
                          <Auditoria />
                        ) : id === "empresas" ? (
                          <Empresas />
                        ) : id === "gafetes" ? (
                          <Gafetes />
                        ) : id === "catalogoRutas" ? (
                          <CatalogoRutas />
                        ) : id === "proveedores" ? (
                          <Proveedores refrescarSenal={refrescarActivos} />
                        ) : (
                          <GafetesProvisionales refrescarSenal={refrescarActivos} />
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
            <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
              <span>{mensajeEstado}</span>
              {accionEstado && (
                <button
                  type="button"
                  // Mismo botón que "Sincronizar" (BarraNube.tsx), para que
                  // la barra se vea pareja.
                  className="barra-estado-boton"
                  onClick={accionEstado.alPulsar}
                  style={{ display: "flex", alignItems: "center", gap: "0.3rem" }}
                >
                  <LogOut size={13} strokeWidth={2} aria-hidden="true" />
                  {accionEstado.texto}
                </button>
              )}
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
              <BarraNube
                sincronizando={sincronizandoManual}
                onSincronizar={sincronizarManualmente}
                estadoConexion={estadoConexionNube}
              />
              <button
                type="button"
                className="barra-estado-boton boton-icono"
                onClick={buscarActualizacionManual}
                disabled={buscandoActualizacion}
                title="Buscar actualización"
                aria-label="Buscar actualización"
              >
                {buscandoActualizacion ? (
                  <Loader2 size={15} strokeWidth={2} className="girando" aria-hidden="true" />
                ) : (
                  <Download size={15} strokeWidth={2} aria-hidden="true" />
                )}
              </button>
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
      </AccionBarraEstadoProvider>
      </BarraEstadoProvider>
    </SesionProvider>
  );
}
