# Handoff: rediseño visual, UX y rendimiento de la app móvil (sesión dedicada)

Instrucciones para **una sesión que trabaja sólo en esto**. Leer completo
antes de tocar nada. Sale de un análisis de sólo lectura de `mobile/`
hecho el 2026-11-14 sobre la versión 1.8.13 (`versionCode` 58): no se
cambió código, todo lo de abajo está por hacer.

Estado: **sin empezar**.

## Documentos de referencia (obligatorio leer)

1. `AGENTS.md`: español; nunca el término prohibido para el puesto de
   control ("puesto de control", "portería" o "punto de acceso"); commit
   documentado + push por cada cambio exitoso; commits sólo a nombre de
   `Daniel Quintana`, sin atribución a asistentes.
2. `mobile/android/arquitectura.md`: Pantalla → ViewModel → Núcleo. La
   fase 4 de este documento existe para cumplirla en `PantallaPrincipal`.
3. `mobile/android/docs/telemetria-diagnostico.md`: los eventos `frames`,
   `pantalla`, `arranque` y `llamada_nucleo` con los que se mide la fase 5.
4. `design/README.md`: el móvil **no** sale de `design/brisas.json` desde
   2026-09-15; sus colores viven a mano en `PaletaLattis.kt`.

## Rama y límites

Crear una rama por fase desde `main` (por ejemplo
`movil/fase-1-color`). Un PR por fase.

| Esta sesión **sí** toca | Esta sesión **no** toca |
|---|---|
| `mobile/android/**` | `src/**` (núcleo raíz), `supabase/**` |
| `mobile/rust-core/**` sólo para el punto 5.6 | `desktop/**`, `web/**`, `web-visitas/**` |
| `docs/manual/**` si cambia algo que el manual describe | `design/**` |

Ninguna regla de negocio cambia: todo es presentación, organización de
Kotlin y rendimiento. Si algo obligara a tocar el núcleo raíz, **parar y
avisar al dueño**.

Los archivos Kotlin que se nombran están en
`mobile/android/app/src/main/java/com/brisas/controlacceso/`.

## Diagnóstico

La base está bien armada y ya optimizada: perfil base, R8, tubería de
cámara con descarte de frames, listas con clave estable, animaciones en la
fase de dibujo, configuración de estabilidad de Compose. **No hay una
ganancia grande de rendimiento escondida.** Lo que más rinde es
simplificar el lenguaje visual y la pantalla principal.

Criterio de todo el trabajo (menos es más, navaja de Ockham):

> **El azul dice qué puedo tocar. Verde, ámbar y rojo dicen qué pasó. El
> color de sección sólo dice dónde estoy.**

### Mediciones del tema claro actual (WCAG)

| Qué | Hoy | Por qué importa |
|---|---|---|
| Fondo `#D0D5E3` contra tarjeta `#E1E2E7` | 1,13:1 | Las tarjetas casi no se separan del fondo, peor al sol |
| Número de gafete sobre su cuadro | 3,10:1 (Contratistas), 2,95:1 (Proveedores) | Es el dato que se coteja en la salida y no llega a 4,5:1 |
| Etiqueta de pestaña (11sp) en acento | 3,55:1 (Proveedores), 3,78:1 (Contratistas) | Texto chico por debajo de 4,5:1 |
| Texto secundario sobre `variante` | 3,72:1 | Por debajo de 4,5:1 |
| Rojo KOF `#C4161C` contra rojo de error `#A93737` | 1,06:1 entre sí | Un botón "Entregar" rojo se lee como destructivo o error |
| Naranja Proveedores `#B85C00` contra ámbar "adentro" `#A87400` | 1,13:1 entre sí | Dos significados con el mismo color |

El tema oscuro contrasta bien (texto 14,7:1, secundario 6,4:1) y casi no
se toca.

### Otros hallazgos, con su evidencia

- **Color con doble significado.** Rojo = sección KOF, error, bloqueado y
  "Cerrar sesión". Ámbar/naranja = sección Proveedores, "adentro",
  advertencia, vencido en el escáner. Azul = acento general, salida por
  gafete e información. `TipoEstado.AVISO` y `ERROR` comparten el mismo
  fondo rojo en `FranjaEstado.kt`.
- **La acción principal es la menos visible.** "Registrar ingreso" es un
  botón de vidrio sin texto (`BotonRedondoAccion`), del mismo peso que la
  barra, y cambia de ícono por sección: cámara (Contratistas), `PersonAdd`
  (Proveedores, Correo), `Add` (KOF).
- **Un ícono con dos significados.** `PersonAdd` es "Nuevo contratista"
  en la barra de arriba y "Nuevo ingreso de proveedor" abajo.
- **La lista principal no dice qué hace tocar una fila.** Sólo los
  resultados de búsqueda llevan la pista "Toque para registrar su salida".
- **Tamaños táctiles dispares.** `AlturaControlBrisas` = 38dp en
  "Registrar ingreso/salida" (lo más usado); 48dp sólo en el login
  (`AlturaBotonPorteria`). Botones de ícono de arriba: 36dp.
- **Dos lenguajes de formulario.** 12 `OutlinedTextField` y 10
  `CampoAgrupado`.
- **Tres familias de íconos.** Material relleno (55 usos), Material
  contorno (10) y Lucide (10, `IconosLucide.kt`).
- **Doce radios distintos**: 2, 8, 10, 11, 12, 14, 16, 18, 20, 26, 28 y
  cápsula. **Espacios fuera de escala**: 5dp (22 usos), 6dp (35), 10dp
  (26), 14dp (13).
- **Quedan 4 `Toast`** junto a `AvisosBreves` (`PantallaPrincipal.kt` ×3,
  `MenuCuenta.kt` ×1).
- **`PantallaPrincipal.kt` tiene 1.026 líneas** y unas 400 no dibujan:
  sincronización, canal en vivo, bloqueo por inactividad, permisos y
  conteos viven en el `@Composable`, con `rememberCoroutineScope`. La
  navegación son cuatro booleanos (`mostrarAvisos`,
  `mostrarNuevoContratista`, `mostrarAyuda`, `mostrarReconexion`) y el
  "apagar tres, prender uno" se repite seis veces.
- **Destello blanco al arrancar en oscuro.** El manifiesto usa
  `@android:style/Theme.Material.Light.NoActionBar` y no existe
  `res/values/`: la ventana y la pantalla de inicio de Android 12+ salen
  blancas aunque el tema sea oscuro.
- **Refresco global por cada aviso en vivo.** `NubeRealtime` llama
  `onCambioAplicado()` sin decir qué tabla cambió; `refrescarNube += 1`
  recarga las cuatro secciones (ocho lecturas al núcleo, todas por el
  mismo `Mutex<AppCore>`).
- **El carrusel compone la página vecina recién al deslizar**
  (`HorizontalPager` sin `beyondViewportPageCount`).

## Fase 1 — Color

Archivos: `PaletaLattis.kt`, `AcentoSeccion.kt`, `FranjaEstado.kt`,
`FilasActivos.kt`, `ControlesBrisas.kt`, `DisenoMovil.kt` y los tests
`PaletaLattisTest`, `AcentoSeccionTest`.

### Paleta clara propuesta (reemplaza Tokyo Day)

| Rol (`Paleta`) | Color | Contraste medido |
|---|---|---|
| `fondo` | `#F2F4F7` | — |
| `tarjeta` | `#FFFFFF` | — |
| `variante` (campos, cuadro de gafete) | `#E8ECF2` | — |
| `realce` | `#FFFFFF` | — |
| `texto` | `#111827` | 17,7:1 sobre tarjeta |
| `textoSecundario` | `#4B5563` | 7,6:1 sobre tarjeta; 6,9:1 sobre fondo |
| `bordeSuave` / `borde` | `#D0D5DD` / `#667085` | 5,0:1 el fuerte sobre tarjeta |
| `acento` y `relleno` | `#1D4ED8` | 6,7:1 con texto blanco y sobre tarjeta |
| `acentoSuave` | `#E0E7FF` | 5,4:1 con el acento encima |
| `exito` / fondo suave | `#15803D` / `#DCFCE7` | 5,0:1 sobre tarjeta |
| `textoAdvertencia` / fondo suave | `#92400E` / `#FEF3C7` | 6,4:1 sobre su fondo |
| `error` / `errorSuave` | `#B42318` / `#FEE4E2` | 5,5:1 sobre su fondo |

Los tonos de sección se conservan sólo para la pestaña activa: sobre
tarjeta blanca dan Contratistas `#00806A` 4,9:1, Proveedores `#B85C00`
4,6:1, KOF `#C4161C` 6,0:1 y Correo `#6D32C9` (ya pasaba).

Paleta oscura: se queda. Único cambio: `relleno` = `#2F5FD0` (5,7:1 con
blanco) para todos los botones.

### Tareas

- [ ] 1.1 Reemplazar `PaletaDia` por la tabla de arriba.
- [ ] 1.2 `PaletaNoche`: `relleno` y `rellenoInfo` a `#2F5FD0`.
- [ ] 1.3 `TemaSeccion` deja de reemplazar `primary`, `primaryContainer`
      y compañía; quitar `LocalRellenoBotones` y sus lecturas
      (`BotonBrisas`, `BotonSinGafete`). Botones, enlaces, cursor y foco
      quedan en el azul general.
- [ ] 1.4 El color de sección se usa sólo en `BarraSecciones` (ícono y
      etiqueta de la pestaña activa), que ya lo lee con `acentoDeSeccion`.
- [ ] 1.5 `CuadroGafete` (`FilasActivos.kt`): texto `onSurface` sobre
      `surfaceVariant`, y subir el número de 15sp a 17sp.
- [ ] 1.6 "Adentro" deja de ser ámbar: `FilaContratista` usa
      `tinteSalida` (azul suave) para quien ya está adentro. Ámbar queda
      sólo para advertencias (PRAIND por vencer, sin subir, hora de otro
      día); rojo sólo para bloqueo y error. Revisar `SenalAdentro`: sin el
      fondo ámbar, decidir si se cambia por un ícono de salida en azul.
- [ ] 1.7 `coloresEstado` en `FranjaEstado.kt`: `AVISO` en ámbar
      (`textoAdvertencia` sobre su fondo suave), `ERROR` en rojo. Lo mismo
      en `HostAvisosBreves`: agregar `rellenoAviso` o reutilizar el ámbar.
- [ ] 1.8 Borrar de `Paleta` lo que sobra tras lo anterior:
      `TonoSeccion.relleno`, `adentroSuave`, `franjaAdentro`,
      `textoAdentro`, `tinteAdentro`, `acentoInverso`, y `rellenoInfo` si
      queda igual a `relleno`.
- [ ] 1.9 Tests de contraste: 4,5:1 para todo texto (incluida la etiqueta
      de pestaña y el gafete) y 3:1 para íconos, en los dos temas.
- [ ] 1.10 Probar en el teléfono a pleno sol, claro y oscuro, las cuatro
      secciones, la hoja de ingreso, la de salida y el escáner.

No se tocan `ColorEscaneo*` (el marco de la cámara va sobre la imagen y
es igual en los dos temas).

## Fase 2 — Que la herramienta se explique sola

- [ ] 2.1 `BotonRedondoAccion` (`AccionInferior.kt`): relleno `relleno`
      con ícono `sobreRelleno`, en lugar de vidrio. Es la acción principal
      de la app y debe ser lo más visible de la barra.
- [ ] 2.2 Un solo ícono para "nuevo ingreso" en las cuatro secciones. En
      Contratistas el ingreso empieza por la cámara: decidir con el dueño
      si se unifica en "+" o si la cámara se queda sólo ahí.
- [ ] 2.3 Quitar "Nuevo contratista" (`PersonAdd`) de la barra de arriba
      en `PantallaPrincipal.kt`. El alta ya se ofrece cuando la búsqueda
      no encuentra a nadie (`FilaCrear` en `ResultadosIngreso`); dejarla
      además como opción del `MenuCuenta`. Arriba quedan título, campanita
      y cuenta. **Confirmar con el dueño antes de sacarlo.**
- [ ] 2.4 Renglón fijo sobre cada lista de adentro (`ListaConEstados`):
      "12 adentro · toque a alguien para registrar su salida" (en KOF,
      "…su devolución").
- [ ] 2.5 `AlturaControlBrisas` de 38dp a 48dp (`DisenoMovil.kt`);
      `BotonDiscretoBrisas` puede quedar en 40dp. Revisar que la hoja de
      ingreso siga entrando con el teclado abierto en el A25.
- [ ] 2.6 `TAMANO_BOTON_CUADRADO` de 36dp a 44dp.
- [ ] 2.7 Un solo estilo de campo: migrar los 12 `OutlinedTextField` a
      `CampoAgrupado`/`GrupoCampos` (hoja de ingreso, formularios de
      Proveedores, Correo, Rutas y Nuevo contratista). Borrar
      `ColoresCampoBrisas` al terminar.
- [ ] 2.8 Reemplazar los 4 `Toast` por `AvisosBreves` donde haya
      `LocalAvisosBreves` (el "Toque de nuevo para salir" puede quedar
      como aviso breve también).
- [ ] 2.9 Una sola familia de íconos: pasar los ~45 íconos de Material a
      `IconosLucide.kt` y quitar `material-icons-extended` de
      `app/build.gradle.kts`.
- [ ] 2.10 Tipografía mínima: `labelSmall` de 11sp a 12sp, `bodySmall` de
      13sp a 14sp (`TipografiaBrisas`). Comprobar que las cuatro etiquetas
      de pestaña entren en una línea.
- [ ] 2.11 Actualizar `docs/manual/` si alguna captura o paso describe lo
      que cambió.

## Fase 3 — Formas, espacios y movimiento

- [ ] 3.1 Tres radios en `DisenoMovil.kt`: 8dp (cuadros chicos), 12dp
      (controles y campos), 20dp (tarjetas y hojas), más el círculo.
      Reemplazar todo `RoundedCornerShape(n.dp)` suelto por esos tres.
      Las piezas cortadas de abajo (`FormaPiezaInicio/Fin`) se simplifican
      a una sola forma si 3.2 las deja sin sentido.
- [ ] 3.2 Barra de secciones y píldora de búsqueda opacas: quitar
      `ALFA_VELO`, `bordeVidrio` y `ANCHO_BORDE_VIDRIO`; usar `bordeSuave`.
- [ ] 3.3 Escala de espacios 4, 8, 12, 16, 24: reemplazar los 5, 6, 10 y
      14dp. Separación entre tarjetas: 8dp.
- [ ] 3.4 Quitar animaciones decorativas: el rebote del ícono en
      `BotonRedondoAccion`, `FilaEscalonada` y el resorte de `MenuCuenta`,
      y la entrada de la flecha de `IconoPaso` (con ella se va
      `rememberHojaArriba`, que espera cuadro a cuadro a que la hoja se
      asiente). `SenalAdvertencia` queda quieta.
- [ ] 3.5 Se conservan: `transicionDePantalla`, la pastilla que sigue al
      dedo, `animateItem`, el aviso que baja y `sacudirAl`.
- [ ] 3.6 Duraciones sólo `MS_SALIDA` (120), `MS_FUNDIDO_ENTRADA` (180) y
      `MS_ENTRADA` (240); quitar `DURACION_FUNDIDO_PILDORA_MS` (300) y
      `MS_FLECHA` (320).
- [ ] 3.7 Decidir con el dueño si `NieveNavidena.kt` se borra (hoy
      apagada con `NAVIDAD_ACTIVA = false`, junto con
      `scripts/icono_navideno.py`).

## Fase 4 — Organización interna

- [ ] 4.1 `PantallaPrincipal.kt`: una sola
      `var panel by rememberSaveable { mutableStateOf(PanelPrincipal.PESTANAS) }`
      en lugar de los cuatro booleanos. El `BackHandler` de la raíz queda
      `enabled = panel == PanelPrincipal.PESTANAS`.
- [ ] 4.2 Nuevo `SesionViewModel` con lo que hoy orquesta el composable:
      `NubeRealtime`, `SincronizacionPeriodica`, el bloqueo por
      inactividad (`DEMORA_BLOQUEO_SESION_MS`), `equipoRetirado`,
      `mensajesConflictos`, `refrescarNube` y la revisión de la bandeja.
      Usa `viewModelScope`; la pantalla sólo le avisa `ON_START`/`ON_STOP`.
      Con tests como los demás ViewModel (`NucleoDePrueba`).
- [ ] 4.3 Sacar `BarraSecciones` y `CampanitaAvisos` a archivos propios.
      Meta: `PantallaPrincipal.kt` por debajo de ~300 líneas, sin leer
      `paginas.currentPage` ni `sincronizando` en el cuerpo de la función
      raíz (bajarlos a los hijos que los usan).
- [ ] 4.4 Renombrar `SeccionPrincipal.ACTIVOS` a `CONTRATISTAS`.
- [ ] 4.5 Borrar `AvatarBrisas` (`ControlesBrisas.kt`), que no tiene
      usos. Comprobado el 2026-11-14: `ListaConDesvanecido` (Rutas,
      Visitas, `PasosRuta`), `transicionDePestana` (`PantallaAvisos`) y
      `GestorTema.alternar` (`PantallaLogin`) **sí se usan**, no borrarlos.
- [ ] 4.6 Actualizar `mobile/android/arquitectura.md` con el
      `SesionViewModel` y la navegación por enum.

## Fase 5 — Rendimiento (medir antes y después)

Medir con el build de diagnóstico y los eventos `arranque`, `pantalla`,
`frames` (p90 y trabados por pantalla) y `llamada_nucleo`. Anotar los
números de antes en el PR.

- [ ] 5.1 Arranque sin destello: crear `res/values/themes.xml` y
      `res/values-night/themes.xml` con un tema propio cuyo
      `android:windowBackground` sea el fondo de la app en cada modo, y
      usarlo en el manifiesto. Seguro y barato.
- [ ] 5.2 Refresco por tabla: `onCambioAplicado(tabla)` en `NubeRealtime`
      y refrescar sólo la sección de esa tabla; el refresco completo
      queda para la sincronización. Seguro.
- [ ] 5.3 Comprobar con `-PinformesCompose=true` que, tras 4.2 y 4.3, la
      raíz ya no se recompone en cada cambio de sección o sincronización.
- [ ] 5.4 **Experimento**: `beyondViewportPageCount = 1` en el
      `HorizontalPager`. Se queda sólo si baja el p90 de `frames` al
      deslizar y no sube `pantalla` de "principal".
- [ ] 5.5 **Experimento**: perfil base generado con Macrobenchmark en
      lugar del comodín de `baseline-prof.txt`. Se queda sólo si mejora
      `arranque`.
- [ ] 5.6 Opcional: una función del núcleo móvil que resuelva varios
      gafetes en un cruce, en lugar de un `listarIngresosActivos` por
      número (`ActivosViewModel.filaPorGafete`). Ganancia chica.

## Orden

1. Fase 1, puntos 2.1 a 2.6 y 5.1 (lo que nota quien opera).
2. Fase 4, puntos 4.1 a 4.3 (abarata lo que sigue).
3. Fase 3 y el resto de la fase 2.
4. Fase 5, puntos 5.2 a 5.5, con medición.

## Decisiones que son del dueño

- 2.2: ¿un solo ícono de "nuevo ingreso" o la cámara se queda en
  Contratistas?
- 2.3: ¿se saca "Nuevo contratista" de la barra de arriba?
- 1.3: el color por sección se pidió el 2026-10-19 porque las secciones
  se confundían. Este plan lo conserva sólo en la pestaña; si en la prueba
  real vuelven a confundirse, sumar el color al título de arriba antes de
  volver a pintar botones.
- 3.7: ¿se borra la nieve navideña?

## Verificación de cada PR

- `./gradlew :app:assembleDebug` y `./gradlew :app:testDebugUnitTest` en
  `mobile/android/` (los tests compilan el núcleo para el host).
- Prueba manual en el Samsung A25: ingreso y salida de contratista,
  proveedor, correo y KOF, con y sin escáner, en claro y oscuro.
- Subir `versionCode`/`versionName` sólo cuando el dueño pida publicar.
