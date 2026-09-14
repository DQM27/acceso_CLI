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

**Fase 4 — `CampoFechas.tsx` accesible (hecho 2026-09-13)**: nuevo componente con dos `<input type="date">` nativos + `SelectorFechas` embebido, sincronizados en ambas direcciones; `dominio.ts` (`validarRangoFechas` extraída, reutilizada por `esquemaNuevaCita` y por la validación en vivo del componente). El reskin de FullCalendar no hizo falta (ya heredaba el tema desde la Fase 1). Nuevo test e2e "las fechas se completan 100% por teclado, sin tocar el calendario" -- 16/16 Playwright, 30/30 vitest, 0 errores de eslint.

**Fase 5 — Wizard de 3 pasos (base hecha 2026-09-13, falta `Activity`/`ViewTransition`)**: nuevo `componentes/PasoWizard.tsx` (numerado, línea conectora, visible en todos los breakpoints), nuevo `lib/useFocoAlCambiar.ts`, `NuevaCita.tsx` reestructurado en 3 pasos (`cuando-donde`/`visitantes`/`revision`) con filtrado de errores por paso reutilizando `esquemaNuevaCita().safeParse()` completo (sin sub-esquemas) — se encontró y corrigió en el camino un bug real de UX: al avanzar de paso se mostraban de entrada los errores de campos del paso siguiente que el usuario todavía no había tocado; ahora se limpian los errores del paso de destino al llegar. `e2e/visitas.spec.ts` reescrito (flujo pasó de 1 click a 2 "Continuar" + 1 "Confirmar") — 17/17 Playwright (2 proyectos, incluye aserción de que el stepper es visible en `movil`), 30/30 vitest, 0 errores de eslint. Falta envolver los pasos en `<Activity>` y animar la transición con `<ViewTransition>`.

**Fase 6 — Escalado de "Visitantes" para grupos grandes**: nuevo `componentes/VisitanteFormulario.tsx`, `<details>/<summary>` (o `.accordion` de Bootstrap, sin JS de Bootstrap — controlado por React) para colapsar tarjetas completas con 4+ visitantes.

**Fase 7 — Rediseño de "Mis citas"**: toolbar con `.nav-pills`/`.btn-group`, tarjeta-clicable, `ViewTransition` en el toggle Lista/Calendario. Actualizar locators de `e2e/visitas.spec.ts`.

**Fase 8 (opcional, gated a evidencia real)**: si las pruebas manuales en dispositivo muestran que el teclado en pantalla tapa un input (patrón `100dvh`+`overflow:hidden` en el shell), relajar `overflow` sólo bajo `@media(max-width:800px)`.

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
- `web-visitas/src/componentes/SelectorFechas.tsx` (se reutiliza, no se reescribe)
- `web-visitas/src/pantallas/MisCitas.tsx`
- `web-visitas/src/api.ts`
- `web-visitas/e2e/visitas.spec.ts`
