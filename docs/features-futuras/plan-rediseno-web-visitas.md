# Rediseño UX/UI/accesibilidad de `web-visitas` sobre Bootstrap — plan (borrador, en curso)

> Documento de continuidad para retomar esta conversación en otra sesión.
> Nace de una auditoría completa de UX/accesibilidad de `web-visitas` y una
> investigación de buenas prácticas (GOV.UK Design System, USWDS, WCAG
> 2.1/2.2 AA, Nielsen Norman Group/Baymard, RAE lenguaje claro), cruzada
> con varias rondas de decisiones explícitas del dueño de producto. La
> Fase 0 (`npm install` + dependencias) ya se ejecutó; el resto sigue sin
> implementar.

## Contexto

`web-visitas` es la app donde un **anfitrión** (usuario final no técnico, login con Google) agenda citas para que sus visitantes entren después a un sitio físico — el cliente real de este proyecto es **FEMSA** (la embotelladora de Coca-Cola). Es un target y un dueño completamente distintos a los de la app desktop (esa es para el operador entrenado del sitio, ya resuelta, fuera de alcance).

El dueño de producto pidió evitar "una montaña de botones que no se entienden", priorizar accesibilidad y claridad, que se sienta natural (no forzada), y —tras varias rondas de ida y vuelta— tomó estas decisiones de arquitectura visual:

1. **Migrar a Bootstrap** como base de componentes visuales — prefiere controles ya probados ("no es inventar, es usar material de calidad") en vez de seguir con CSS hecho enteramente a mano. Explícitamente no quiere "webs genéricas" ni algo con pinta de plantilla sin criterio.
2. **Acento rojo tipo Coca-Cola, sin logo** — "los colores no le pertenecen a nadie, son libres" (correcto: un color plano no es la marca registrada; lo que sí está fuera de alcance es el logo, el guion/script tipográfico de marca, o cualquier otro elemento de trade dress). Se usa como color de marca/CTA (`$primary` de Bootstrap), no "rojo por todos lados".
3. **Inspiración de interacción de sitios de reserva reales** (Avianca): patrón de barra de búsqueda compuesta (ícono + etiqueta chica + valor, varios campos en una sola tarjeta) y toggles tipo píldora — patrones de layout genéricos y muy probados, sin relación de marca con Avianca.
4. **Wizard "lo justo y necesario"** — ni todo en una pantalla, ni fragmentado al extremo estilo GOV.UK puro.
5. **Usar las novedades de React 19.3** (release 2026-09-09) donde aporten valor real: `<Activity>` y `<ViewTransition>` en el wizard (ver más abajo) — confirmado contra el API oficial de react.dev, no es un adorno.

**Nota técnica de partida**: el sitio en producción (`visitas.megabrisas.com`, vía Cloudflare Workers/`wrangler.jsonc`) es una SPA renderizada en cliente — no se pudo inspeccionar su versión actual con `WebFetch` (devuelve el HTML crudo, vacío antes de que React lo pinte); si el dueño de producto quiere que se reutilice algo puntual de esa versión visible hoy, pedirle una captura de pantalla antes de asumir nada de ese sitio.

---

## Decisión de arquitectura: Bootstrap + Sass, desacoplado de `design/brisas.json`

**Se agrega**: `bootstrap` (npm) + `sass` (devDependency — Vite compila `.scss` sin configuración adicional). Nuevo archivo `web-visitas/src/estilos/tema.scss`:
```scss
// 1) Overrides ANTES de importar Bootstrap (mapea nuestras decisiones a sus variables)
$primary: #E4002B;       // rojo tipo Coca-Cola -- a afinar con revisión de contraste real, sin logo/tipografía de marca
$enable-dark-mode: true; // ya viene en true por defecto en 5.3, se deja explícito
$border-radius: .75rem;  // afín al --radio:12px que ya se venía usando
$font-family-base: "Segoe UI", Roboto, system-ui, sans-serif; // conserva la tipografía actual, no la de marca de Coca-Cola

@import "bootstrap/scss/bootstrap";
```

**`web-visitas/src/index.css`**: se quita `@import "../../design/brisas.css";` y `@import "tailwindcss";` (Tailwind ya estaba instalado sin una sola clase en uso — confirmado en la auditoría). `index.css` pasa a contener sólo overrides puntuales que Bootstrap no cubre (el `<dialog>` nativo, el layout del shell/sidebar, el `SelectorFechas`/FullCalendar).

**Por qué desacoplar de `design/brisas.json`** (la fuente de tokens compartida con `desktop`, `web` operador y `mobile/android`): con Bootstrap como sistema de diseño propio de esta app, ya no tiene sentido heredar esos tokens — evita el riesgo de que un cambio visual acá se propague sin querer a la app del operador. `web-visitas` queda con un sistema de diseño 100% propio (Bootstrap + `tema.scss`), sin ningún import cruzado hacia `design/`.

**Modo oscuro**: se consolida en el atributo propio de Bootstrap, `data-bs-theme` en `<html>` — `componentes/Comunes.tsx` (`SelectorTema`) pasa a setear `document.documentElement.dataset.bsTheme` (en vez del actual `dataset.theme`), con el mismo mecanismo de detección de `prefers-color-scheme` y persistencia en `localStorage` que ya existe hoy. Un solo atributo, sin sistemas de tema paralelos.

**Qué NO se trae**: `bootstrap.bundle.min.js` (evita Popper — dropdown/tooltip/popover no hacen falta para nada de lo que se rediseña acá, y Popper puede escribir `style="..."` como atributo HTML string, lo que arriesgaría la CSP estricta actual sin `unsafe-inline` en `style-src`). Toda la interactividad se queda en estado de React, igual que hoy — sólo cambia qué clases visuales se aplican.

**Modal**: se conserva el `<dialog>` nativo de `Comunes.tsx` (su manejo de foco al abrir/cerrar ya es correcto y más robusto que el modal JS de Bootstrap) pero reestilado con las clases visuales de Bootstrap (`.modal-content`, `.modal-header`, `.modal-body`, `.modal-footer`, `.btn-close`) — se adopta el lenguaje visual de Bootstrap sin perder el comportamiento accesible ya resuelto.

**Contraste del rojo**: Bootstrap calcula automáticamente texto negro/blanco según el fondo (función Sass `color-contrast()`), pero igual hay que verificar a ojo y con una herramienta de contraste real el `$primary` elegido contra fondo blanco Y en modo oscuro — es una verificación obligatoria antes de dar por cerrada la Fase 1, no algo que se asume porque "Bootstrap ya lo resuelve".

**Clases de Bootstrap a EVITAR por la CSP estricta (`img-src 'self'`, sin `data:`)** — confirmado inspeccionando el CSS compilado (`grep data:image` en el build): varios componentes de Bootstrap pintan su ícono vía `background-image: url("data:image/svg+xml,...")`, lo que la CSP bloquea (mismo tipo de problema ya resuelto antes con el ícono de FullCalendar, ver `parcheCspFullcalendar.ts`). Reglas para todo el resto de la migración:
- **`.btn-close`**: no usar. El botón de cerrar sigue siendo nuestro propio ícono `<X>` de `lucide-react` sobre `.btn.btn-link` (icono solo).
- **`.form-check-input`** (apariencia custom de checkbox/radio): no usar su apariencia por defecto. Donde ya existe un indicador hecho a mano (`.sitio-opcion`/`.sitio-indicador` en `NuevaCita.tsx`, ícono `<Check>` propio) se mantiene tal cual -- ya es CSP-safe. Para un checkbox/radio nuevo que no necesite indicador custom, usar el input nativo sin la clase `.form-check-input` y colorearlo con `accent-color: var(--bs-primary)` (una sola propiedad CSS, sin imágenes).
- **`.form-select`**: si se usa un `<select>` nativo en algún paso futuro, agregar `appearance: auto; background-image: none;` para que el navegador dibuje su propia flecha en vez de la de Bootstrap.
- **`.is-invalid`/`.is-valid`**: no usar estas clases. El estado de error se sigue marcando con el atributo `aria-invalid="true"` + un selector CSS propio (`[aria-invalid="true"] { border-color: var(--bs-danger); }`), igual que ya hacía `dominio.ts`/`atributos()` antes de Bootstrap.
- **`.accordion`** (Fase 6, colapsar visitantes): no usar el componente Bootstrap -- seguir con `<details>/<summary>` nativo + ícono propio (`ChevronDown` de lucide), tal como ya preveía el plan por el motivo de evitar `bootstrap.bundle.js`/Popper; ahora hay una segunda razón (el ícono de flecha del acordeón también es `data:image`).

---

## Decisiones de UX

### Wizard de 3 pasos

| Paso | Título | Contenido |
|---|---|---|
| 1 | "¿Cuándo y dónde?" | Sitios, fechas (`CampoFechas` nuevo), hora estimada (nueva), motivo |
| 2 | "Visitantes" | Alta/baja de visitantes |
| 3 | "Revisar y confirmar" | Resumen + confirmar |

El paso 1 se arma como una **barra de búsqueda compuesta** inspirada en Avianca: una `.card` con varios `.input-group` en fila (desktop) / apilados (móvil), cada uno con ícono (`lucide-react`) + label chico + control — para Sitios (multi-select tipo chips/checkboxes dentro de un `.dropdown`-like propio, sin Popper), Fechas (`CampoFechas`), Hora estimada. El indicador de progreso usa `.nav.nav-pills`/badges numerados de Bootstrap en vez de CSS a mano, visible en **todos** los breakpoints (se elimina la regla actual que lo oculta en móvil).

### Selector de fecha accesible, sin duplicar validación

Nuevo `componentes/CampoFechas.tsx`: dos `<input type="date">` nativos con clase `.form-control` (siempre visibles, 100% teclado, picker nativo en móvil) como fuente de verdad primaria, con `SelectorFechas.tsx` (FullCalendar) embebido como complemento visual — **sin reescribir su lógica interna** (`select`/`finExclusivo` se quedan igual), sincronizado en ambas direcciones por el mismo `onCambiar(desde, hasta)` que ya existe. Resuelve el hallazgo de accesibilidad más serio de la auditoría (hoy `SelectorFechas.tsx` sólo se opera con mouse/touch).

Las reglas de fecha (`dominio.ts:50-56`) se extraen a `validarRangoFechas(desde, hasta, hoy)`, reutilizada por `esquemaNuevaCita` y por el `onBlur` de cada input (validación inline, reduce errores ~22% según Baymard).

FullCalendar se reskinéa con sus propias CSS custom properties (`--fc-border-color`, `--fc-button-bg-color`, etc.) mapeadas a las variables `--bs-*` que Bootstrap ya expone, para que no desentone visualmente.

### `hora_estimada`

`<input type="time" className="form-control">`, **sin `min`/`max`** — la migración de Supabase (`20260909215318`/`20260909215400`) es explícita en que "ninguna política ni la RPC la usan para autorizar nada"; imponer un rango sugeriría visualmente una restricción que no existe. Se compensa con copy: label "Hora aproximada de llegada" + badge "Opcional", ayuda "Es solo para orientar al personal del sitio — no es necesario llegar puntual ni se bloquea el ingreso a otra hora."

Cambios: `dominio.ts` (campo opcional en `esquemaNuevaCita`/`citaEsquema`), `api.ts` (`CAMPOS_CITA` + `p_hora_estimada` en `crearCita`, el 7mo parámetro de la RPC ya existe con `default null`), `fecha.ts` (helper `horaLegible`). Se muestra en: resumen del paso 3, fila de `MisCitas.tsx` (sólo si hay valor, badge discreto), modal de detalle, prefijo del título en `CitasCalendario.tsx`.

### Mis Citas: reducir la "montaña de botones"

- Toolbar: filtros de estado como `.nav.nav-pills` (control dominante); toggle Lista/Calendario como `.btn-group` sólo-ícono; "Actualizar" como botón `.btn-link`/ícono solo al extremo (ya hay refresco automático cada 60s/on-visibility).
- Filas de cita: `.card` completa clicable (`<button>` envolviendo fecha/título/meta) para "ver detalles"; "Cancelar cita" como único `.btn.btn-outline-danger` explícito, hermano, sólo en vigentes.

### Accesibilidad puntual

| Hallazgo | Fix |
|---|---|
| Sidebar colapsable sin rol de botón (`Sidebar.tsx:48-52`) | `<button aria-expanded aria-label>` con ícono `PanelLeftClose`/`PanelLeftOpen`, clase `.btn.btn-link` |
| Botón menú móvil sin estado (`App.tsx:110-117`) | `aria-expanded` + `aria-label` dinámico |
| Mensaje de caracteres de control genérico (`dominio.ts:16`) | Reescribir a lenguaje claro con qué hacer |
| Jerga de idempotencia (`NuevaCita.tsx:504-507`, `594-597`) | Reescribir a tono humano |
| Cédula sin ejemplo de formato (`NuevaCita.tsx:369-384`) | `placeholder="Por ejemplo: 1-2345-6789"` |

---

## Usar React 19.3

**`<Activity mode="visible"|"hidden">`** envolviendo cada paso del wizard en vez del swap condicional actual. Al ocultar un paso, React lo pinta `display:none`, destruye sus efectos, pero preserva DOM y estado — el calendario reaparece en el mismo mes al volver atrás, sin remount.

**`<ViewTransition>` + `addTransitionType`** anima la transición entre pasos con la View Transition API nativa:
```js
function siguiente() {
  startTransition(() => { addTransitionType('adelante'); setPaso('visitantes'); });
}
```
```jsx
<ViewTransition enter={{ adelante: 'desde-derecha', atras: 'desde-izquierda' }}
                exit={{ adelante: 'hacia-izquierda', atras: 'hacia-derecha' }}>
  <PasoActual />
</ViewTransition>
```
Degrada de forma elegante en navegadores sin soporte (React aplica el cambio de estado igual, sin animación) — apropiado para una app pública sin control sobre el navegador del visitante. Se aplica también al toggle Lista/Calendario de `MisCitas.tsx`.

---

## Fases

**Fase 0 — Setup (hecho 2026-09-13)**: `npm install`, agrega `bootstrap` + `sass`, quita `tailwindcss`/`@tailwindcss/vite` del `package.json`/`vite.config.ts`.

**Fase 1 — Fundación visual Bootstrap (hecho 2026-09-13)**: `estilos/tema.scss` con el rojo `$primary` y overrides mínimos; `index.css` reducido a lo que Bootstrap no cubre; quitado el import de `design/brisas.css`; dark mode consolidado en `data-bs-theme` (`Comunes.tsx`); los 7 archivos (`Comunes.tsx`, `Login.tsx`, `App.tsx`, `Sidebar.tsx`, `MisCitas.tsx`, `NuevaCita.tsx`, `CitasCalendario.tsx` -- este último sin cambios, hereda el tema solo) migrados a clases de Bootstrap; puente temporal `.boton*` borrado. Verificado con capturas de Playwright (login, Mis Citas con/sin citas, modal de detalle, Nueva Cita completo incluida la revisión, los 3 en claro/oscuro), `npm run build`/`test`/`lint` en verde.

**Fase 2 — Accesibilidad puntual (hecho 2026-09-13, resuelta junto con la Fase 1)**: Sidebar (botón real con `aria-expanded`), botón menú móvil (`aria-expanded` + label dinámico + ícono que alterna), placeholder de cédula, mensajes de error/copy reescritos en lenguaje claro.

**Fase 3 — `hora_estimada` end-to-end (hecho 2026-09-13)**: `dominio.ts` (`horaOpcional` en `esquemaNuevaCita`, campo requerido -nullable en `citaEsquema`), `api.ts` (`CAMPOS_CITA` + `p_hora_estimada`), `fecha.ts` (`horaLegible`), `NuevaCita.tsx` (`<input type="time">`, resumen lateral + revisión), `MisCitas.tsx` (fila + modal de detalle), `CitasCalendario.tsx` (prefijo del título). 30 tests de vitest (3 nuevos) y 14 de Playwright (1 nuevo) en verde, incluida la aserción real de CSP.

**Fase 4 — `CampoFechas.tsx` accesible (hecho 2026-09-13)**: nuevo componente + `SelectorFechas` embebido, sincronizados en ambas direcciones; `dominio.ts` (`validarRangoFechas` extraída, reutilizada por `esquemaNuevaCita` y por la validación en vivo del componente). El reskin de FullCalendar no hizo falta (ya heredaba el tema desde la Fase 1).

**Ajuste posterior (2026-09-13)**: se probó primero con dos `<input type="date">` nativos, pero el dueño de producto pidió garantizar el orden día/mes/año (estándar de Costa Rica) -- un `<input type="date">` nativo muestra el orden según el idioma/región del NAVEGADOR de cada visitante, no según el `lang` de la página, así que no hay forma de garantizarlo con un solo input nativo. Se reemplazó por 3 campos de texto fijos (Día/Mes/Año), el patrón que recomiendan GOV.UK Design System y USWDS justo para este problema (ya citado en la investigación de buenas prácticas del inicio de este plan). Cada segmento se guarda en estado local del componente (no en el valor `YYYY-MM-DD` del padre) para permitir estados intermedios mientras se tipea sin propagar una fecha inválida; recién avisa al padre cuando los 3 segmentos forman una fecha real (incluye rechazar días que no existen, ej. 31 de febrero). Sin avance automático de foco entre los 3 campos a propósito (GOV.UK tampoco lo hace: sorprende a quien usa lector de pantalla).

Nuevo test e2e "las fechas se completan 100% por teclado, sin tocar el calendario" (actualizado a los 3 campos) -- 20/20 Playwright, 30/30 vitest, 0 errores de eslint.

**Fase 5 — Wizard de 3 pasos + `Activity`/`ViewTransition` (hecho 2026-09-13)**: nuevo `componentes/PasoWizard.tsx` (numerado, línea conectora, visible en todos los breakpoints), nuevo `lib/useFocoAlCambiar.ts`, `NuevaCita.tsx` reestructurado en 3 pasos (`cuando-donde`/`visitantes`/`revision`) con filtrado de errores por paso reutilizando `esquemaNuevaCita().safeParse()` completo (sin sub-esquemas) — se encontró y corrigió en el camino un bug real de UX: al avanzar de paso se mostraban de entrada los errores de campos del paso siguiente que el usuario todavía no había tocado; ahora se limpian los errores del paso de destino al llegar.

**Cambio no planeado, pero necesario**: `componentes/SelectorFechas.tsx` se reescribió de cero, **sin FullCalendar** -- al envolver los 3 pasos en `<Activity>` para probar la preservación de estado, se confirmó en la práctica (no sólo en teoría) que el wrapper de FullCalendar reinicializaba su vista cada vez que Activity volvía a correr sus efectos al mostrar un paso oculto (el DOM sobrevivía, pero el efecto de inicialización no) -- perdía el mes al que el usuario había navegado. Se reemplazó por un calendario propio (grilla de 6 semanas, dos-clicks para elegir rango en vez de arrastrar, mismo formato de fecha en español) que guarda el mes mostrado en `useState` normal, sin ese problema. Beneficio adicional: se eliminó `@fullcalendar/interaction` (dependencia completa, sin más usos) y el parche de CSP de `SelectorFechas.tsx` ya no hace falta ahí (`CitasCalendario.tsx`, en Mis Citas, sigue usando FullCalendar sin cambios -- no tenía este problema porque no está envuelto en `Activity`).

Verificado con un test e2e dedicado ("Activity conserva el mes del calendario al ir y volver entre pasos") que primero **falló de verdad** contra la versión con FullCalendar (confirmando el problema real, no hipotético) y pasa limpio con el calendario propio. `e2e/visitas.spec.ts` reescrito para el flujo de 3 pasos (2 "Continuar" + 1 "Confirmar" en vez de 1 "Revisar cita") -- 18/18 Playwright (2 proyectos, incluye la aserción de Activity y de que el stepper es visible en `movil`), 30/30 vitest, 0 errores de eslint.

**Fase 6 — Escalado de "Visitantes" para grupos grandes (hecho 2026-09-13)**: nuevo `componentes/VisitanteFormulario.tsx` con `<details>/<summary>` nativo (sin `.accordion` de Bootstrap -- su flecha también es `background-image: data:`, bloqueada por la CSP). Con más de 3 visitantes, cada tarjeta se colapsa a una línea de resumen (nombre · cédula) una vez que está COMPLETA -- se descartó colapsar apenas el campo "no está vacío": eso plegaba la tarjeta a mitad de tecleo (el nombre ya no vacío, la cédula todavía sí). La última tarjeta agregada y cualquiera con error quedan siempre abiertas. Nuevo test e2e con 6 visitantes que verifica colapso, reapertura manual y que la detección de cédulas duplicadas sigue funcionando igual sin importar cuántos haya. 20/20 Playwright, 30/30 vitest, 0 errores de eslint.

**Fase 7 — Rediseño de "Mis citas" (hecho 2026-09-13)**: la barra de herramientas con `.btn-group` ya había quedado lista en la Fase 1. Se completó con la tarjeta-clicable: toda la fila (fecha + título + meta) es ahora un solo `<button>` ("ver detalles"), con un ChevronRight como affordance visual; "Cancelar cita" queda como único botón explícito, hermano, sólo en vigentes -- baja de "2 controles del mismo peso por fila" a "1 zona clicable + 1 acción diferenciada". `<ViewTransition>` en el toggle Lista/Calendario para un crossfade suave. No hizo falta actualizar los locators existentes de `e2e/visitas.spec.ts` (el `aria-label` del botón contiene "Ver detalles" como substring, que es como Playwright empareja por defecto). 20/20 Playwright, 30/30 vitest, 0 errores de eslint.

**Fase 8 (opcional, gated a evidencia real)**: si las pruebas manuales en dispositivo muestran que el teclado en pantalla tapa un input (patrón `100dvh`+`overflow:hidden` en el shell), relajar `overflow` sólo bajo `@media(max-width:800px)`.

**Bug real encontrado tras el primer despliegue a producción (2026-09-13)**: con las 7 fases completas se desplegó `web-visitas` a `visitas.megabrisas.com` vía `wrangler deploy` para verificación en vivo. "Sitios de la visita" falló de entrada con "No pudimos completar la solicitud" (el mensaje genérico de `mensajeError` en `api.ts`, que cubre cualquier error de Postgrest sin código especial). Causa: la migración `20260912060901_elimina_direccion_de_sitios.sql` (de trabajo previo, no de este rediseño) eliminó la columna `sitios.direccion` en la base real, pero `web-visitas` nunca se actualizó para dejar de pedirla -- un desacople preexistente entre dominios (esa migración se hizo pensando en el panel del operador y en `mobile`/`desktop`, sin tocar `web-visitas`) que quedó latente hasta este primer despliegue real. Afectaba tanto `listarSitios()` como el `select` anidado de `CAMPOS_CITA` (`cita_sitios(sitio_id,sitios(id,nombre,direccion))`), es decir tanto "Nueva cita" como "Mis Citas" habrían fallado igual. Corregido quitando `direccion` de ambos `select` en `api.ts`, de `sitioEsquema` en `dominio.ts`, y del `<small>{sitio.direccion}</small>` ya muerto en `NuevaCita.tsx`; fixtures de `e2e/visitas.spec.ts` actualizados a juego. Re-verificado: 20/20 Playwright, 30/30 vitest, 0 errores de eslint, redesplegado.

**Lección**: una auditoría de "desincronización entre dominios" hecha antes de tocar código (como la de este plan) puede quedar desactualizada por migraciones de otro trabajo que se mergean a `main` mientras el rediseño está en curso en una rama aparte -- vale la pena repetir un chequeo rápido de columnas/RPCs usadas justo antes de un despliegue real, no sólo al principio.

**Segundo bug real encontrado en producción, mismo día**: el dueño de producto reportó que "Mis Citas" se "reseteaba a cada rato" y se veía feo. Causa: el `useEffect` de refresco de `MisCitas.tsx` corría con `setCargando(true)` en cada revalidación -- no sólo en la carga inicial, también en el `setInterval` de 60s y en el `visibilitychange` (volver a la pestaña, alt-tab). Eso reemplazaba toda la lista/calendario por el spinner de página completa y disparaba el `ViewTransition` cada vez, así que la pantalla completa "parpadeaba" varias veces por minuto sin que el usuario hiciera nada.

Al investigar el porqué del polling, surgió una pregunta más de fondo (planteada por el dueño de producto): las citas las crea, edita y cancela *sólo* el propio anfitrión que las mira -- confirmado en las políticas RLS de `public.citas` (sólo existe una política de escritura, "anfitrion actualiza sus propias citas"; ni el operador de sitio ni ningún dispositivo tienen permiso de escritura ahí, sólo de lectura). No hay ningún actor externo cambiando estos datos en vivo, a diferencia de `desktop`/`mobile` donde sí hace falta sync en tiempo real con lo que hace el guardia del sitio. Conclusión: el `setInterval(60_000)` a ciegas no tenía ninguna razón real de ser -- se quitó por completo. El refresco al volver a la pestaña (`visibilitychange`) sí se conserva, porque cubre un caso real aunque menos común: el mismo anfitrión con dos pestañas o dispositivos (ej. cancela desde el celular mientras tenía la laptop abierta).

Corregido separando "carga inicial / cambio real de filtro o página" (sigue mostrando el spinner de página completa, es lo esperable) de "revalidación de fondo" (ya no reemplaza la lista visible -- sólo hace girar el ícono de "Actualizar"; y si la revalidación de fondo falla, se ignora en silencio en vez de tapar la lista con un aviso de error, total se reintenta solo al volver a la pestaña). Mismo tratamiento para la vista Calendario. Re-verificado: 20/20 Playwright, 30/30 vitest, 0 errores de eslint, redesplegado.

---

## Riesgos

- **Fase 1 es la de mayor superficie mecánica** (toca casi todos los componentes) pero de riesgo relativamente bajo por ser mayormente renombrado de clases — el riesgo real está en no verificar bien el contraste del rojo antes de avanzar.
- **`ViewTransition`/`Activity`** (React 19.3, muy reciente): si algo no se comporta como documentado, el fallback seguro es quitar el wrapper y volver al swap condicional plano.
- **Reescritura de `e2e/visitas.spec.ts` en Fase 5**: mayor punto de quiebre potencial — commit aislado.
- **Deliberadamente sin `bootstrap.bundle.js`/Popper**: si más adelante hace falta un dropdown/tooltip real de Bootstrap, hay que revisar la CSP antes de sumarlo (no asumir que es gratis).
- **Rojo tipo Coca-Cola sin logo**: alcance acotado a un color plano (`$primary`) — no se reproduce el logo, el guion tipográfico de marca, ni la silueta de botella. Si en algún momento se quiere ir más allá (logo, tipografía), eso sí requiere autorización explícita del cliente antes de implementarlo.

---

## Verificación

**Automatizada**: tests nuevos en `pruebas/dominio.test.ts` (`hora_estimada`, `validarRangoFechas`), `pruebas/api.test.ts` (`p_hora_estimada` en el payload), tests de componente para `CampoFechas`/`VisitanteFormulario`, reescritura + casos nuevos en `e2e/visitas.spec.ts` (teclado, stepper visible en `movil`, grupo grande, `hora_estimada` visible). Mantener la aserción de cero violaciones CSP en cada fase.

**Manual en `npm run dev`, por fase**:
- Contraste real del rojo `$primary` en claro y oscuro (herramienta de contraste, no sólo el auto-cálculo de Bootstrap).
- Wizard completo sólo con teclado, foco visible, resumen de errores recibe foco.
- Lector de pantalla (NVDA/VoiceOver) en cada fase que toque accesibilidad.
- Viewport angosto real, teclado nativo no debe tapar inputs.
- Alternar `data-bs-theme` claro/oscuro en cada pantalla tocada.
- Grupo de 50 visitantes: colapso no rompe validación de duplicados.
- Mis Citas con 0/1/12+ citas, ambas vistas.
- `ViewTransition` en Chrome/Edge y degradación limpia en Firefox/Safari.

---

## Archivos críticos

- `web-visitas/src/estilos/tema.scss` (nuevo)
- `web-visitas/src/index.css`
- `web-visitas/src/componentes/Comunes.tsx`
- `web-visitas/src/pantallas/NuevaCita.tsx`
- `web-visitas/src/dominio.ts`
- `web-visitas/src/componentes/SelectorFechas.tsx` (reescrito sin FullCalendar, ver Fase 5)
- `web-visitas/src/pantallas/MisCitas.tsx`
- `web-visitas/src/api.ts`
- `web-visitas/e2e/visitas.spec.ts`
