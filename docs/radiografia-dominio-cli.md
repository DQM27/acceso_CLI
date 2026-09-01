# Radiografía del dominio — para diseño de `--cli`

Este documento explica **qué decide la aplicación y con qué información**,
en lenguaje de negocio, sin arquitectura de código. Es el material de
referencia para diseñar las escenas y mutaciones de `--cli` — no
sustituye a [`docs/diagrama-logico.md`](diagrama-logico.md) (que documenta
el sistema completo, incluida la TUI clásica) ni a
[`docs/lenguaje-visual-mutaciones.md`](lenguaje-visual-mutaciones.md) (que
documenta el plan técnico de la nueva interfaz). Este es el tercero: **qué
sabe y qué decide la aplicación**, para que el diseño de escenas se apoye en
reglas reales y no en supuestos.

## 1. Quién usa la aplicación

Un operador humano en la portería, con sesión iniciada por cédula +
contraseña. Tres roles: `Root`, `Administrador`, `Operador`. Hoy `Root` y
`Administrador` son funcionalmente idénticos salvo un detalle: sólo ellos
pueden editar la cédula de un contratista ya existente y sólo ellos pueden
activar/desactivar el acceso de un contratista. El `Operador` no ve esas
opciones — el campo directamente aparece bloqueado, no oculto.

Login en `--cli`: cédula y contraseña se escriben en el mismo input,
una tras otra. La cédula se valida contra SQLite al instante; la contraseña
se verifica con Argon2 en segundo plano (puede tardar unos cientos de
milisegundos) para no congelar la interfaz mientras calcula el hash.

## 2. Los 7 comandos y qué hace cada uno

```text
/ingreso  (alias: i)   — registrar la entrada de un contratista
/salida   (alias: s)   — registrar la salida de alguien que está adentro
/activos  (alias: a)   — ver la tabla de quién está adentro ahora mismo
/nuevo    (alias: n)   — dar de alta un contratista nuevo
/editar   (alias: e)   — editar un contratista existente
/ayuda                 — lista de comandos
/cerrarsesion (alias: cs) — cerrar sesión y volver al login
```

Texto sin `/` inicial no es un comando aparte: **ya es la búsqueda**. Es la
acción más frecuente (después de ingresos) y la única sin efectos
secundarios, así que no necesita su propio comando.

Parámetros opcionales sobre `/ingreso`: `G:<número>` (gafete) y
`M:caminando|vehiculo` (medio), en cualquier orden, con prefijos no
ambiguos aceptados para `M:` (`M:c`, `M:v`).

## 3. Cómo se resuelve lo que se escribe

Cada tecla recalcula el estado — no hay "enviar" a mitad de camino. Reglas
del motor de búsqueda:

- Con menos de **2 caracteres** de consulta, no se dispara ninguna
  búsqueda — se muestra la pista de uso, nunca un falso "sin resultados".
- Con 2+ caracteres se consulta contra SQLite (texto corto usa `LIKE`,
  3+ usa búsqueda de texto completo FTS5 — invisible para el usuario, el
  resultado se ve igual).
- Máximo **9 coincidencias** visibles a la vez, navegables con ↑↓.
- **Con una sola coincidencia** en `/salida`, la interfaz salta directo a
  la tarjeta de confirmación — no obliga a elegir de una lista de uno.

Autocompletado (Tab) es siempre predecible y nunca ejecuta una acción por sí
mismo — sólo completa texto (nombre de comando, número de gafete libre,
nombre del medio). Confirmar una acción siempre requiere Enter explícito.

La línea de sugerencias es puramente informativa (no cambia estado): indica
qué se puede escribir ahora — parámetros que faltan, atajos de teclado,
gafetes libres con el prefijo tecleado.

## 4. Decisión de ingreso — la regla central de la app

Al confirmar un `/ingreso`, la aplicación vuelve a evaluar todo desde cero
(nada de lo mostrado mientras se tecleaba queda "reservado"):

| Condición del contratista | ¿Requiere PRAIND vigente? | ¿Requiere gafete? |
|---|:---:|:---:|
| Tipo `PRAIND` | Sí | Sí |
| Tipo `IN_HOUSE` | Sí | No |
| Tipo `POR_CORREO` | No | Sí |
| Tipo `SWAT` | No | No |
| Personal de ruta (cualquier tipo) | Sí | No |

Resultado posible de la evaluación:

- **Permitido** — sin advertencias.
- **Permitido con advertencia** — el PRAIND vence dentro de 30 días.
- **Denegado: sin acceso** — el contratista no tiene acceso autorizado.
- **Denegado: PRAIND vencido**.
- **Denegado: PRAIND sin fecha registrada**.
- **Denegado: empresa inactiva**.
- **Error: ya tiene un ingreso activo** — no se puede entrar dos veces sin
  salir primero.
- **Error: gafete requerido** — el tipo lo exige y no se indicó ninguno.
- **Error: gafete ocupado** — ya lo tiene otro ingreso activo ahora mismo.

Si el contratista no requiere gafete, cualquier `G:` que el operador haya
escrito se descarta silenciosamente — no es un error, simplemente no aplica.

La tarjeta de confirmación (`ResumenIngreso`) muestra el resultado de esta
evaluación *antes* de que el operador confirme con Enter — y sólo deja
confirmar (`ingreso_confirmable`) cuando no hay ningún ✗: sin denegación,
sin ingreso activo previo, y con gafete presente y libre si el tipo lo pide.

## 5. Decisión de salida

- Se busca por nombre/cédula (texto libre) o por gafete exacto (`G:`).
- Con una sola coincidencia activa, salto directo a confirmación.
- **Error: el ingreso ya no está activo** (alguien más ya le registró la
  salida, o cambió de estado entretanto).
- **Error: la salida no puede ser anterior al ingreso** — protección de
  reloj retrocedido en el equipo.
- Al confirmar, el contratista y el gafete quedan libres de inmediato.

## 6. El formulario de contratista (`/nuevo` y `/editar`)

Ocho campos, siempre en este orden, recorridos con ↑↓:

```text
Cédula → Nombre → Empresa → Tipo → Fecha PRAIND → Personal de ruta → Acceso → Confirmar
```

- **Cédula, Nombre, Fecha PRAIND** se escriben como texto (el input de la
  línea de comandos se reutiliza para editar el campo activo).
- **Empresa** no se escribe: Enter abre un selector filtrable aparte;
  elegir una empresa cierra el selector y avanza al siguiente campo.
- **Tipo, Personal de ruta, Acceso** se alternan con Space/←/→, no se
  escriben.
- La fecha PRAIND inserta las `/` automáticamente mientras se teclea
  (`DD/MM/YYYY`).
- **Defaults de alta**: tipo `PRAIND`, no es personal de ruta, acceso
  concedido. La cédula siempre es editable al crear.
- **Editando un contratista existente**: si el operador no tiene permiso
  para cambiar la cédula, el campo arranca directamente en Nombre (el
  campo bloqueado no se salta silenciosamente en la navegación normal, pero
  el punto de entrada evita aterrizar ahí).
- **Confirmar** con errores pendientes no avanza — los errores se marcan
  junto a cada campo (✗) y el operador se queda editando. Sólo sin errores
  pasa a la tarjeta de **Resumen**, donde Enter persiste de verdad y Esc
  regresa a edición sin perder lo tecleado.
- **Nada persiste hasta la tarjeta de Resumen** — buscar, editar campos o
  navegar el selector de empresa nunca toca SQLite.

## 7. Qué es lo único que realmente escribe en SQLite

Sólo tres momentos escriben datos, y siempre tras una confirmación
explícita con Enter sobre una tarjeta de resumen — nunca mientras se teclea:

1. Confirmar `ResumenIngreso` → `registrar_ingreso`.
2. Confirmar `ResumenSalida` → `registrar_salida`.
3. Confirmar la tarjeta de Resumen del formulario → `crear_contratista` o
   `actualizar_contratista`.

Todo lo demás (búsquedas, selección, navegación, autocompletado) es de sólo
lectura y se puede recalcular infinitas veces sin ningún efecto secundario
— relevante para el diseño: **explorar y cambiar de opinión nunca cuesta
nada**, la interfaz puede permitirse ser generosa mostrando alternativas.

## 8. Feedback transitorio

Tras una acción exitosa (ingreso, salida, alta, edición, cierre de sesión),
aparece un mensaje de confirmación que dura **4 segundos** o hasta que el
operador vuelva a escribir — lo que pase primero. No es una alerta que
requiera reconocimiento; es puramente informativo.

Hay exactamente tres niveles de feedback en todo el sistema — no hay un
cuarto nivel intermedio:

```text
Éxito       — acción completada
Advertencia — completada, pero con algo que atender (p. ej. PRAIND por vencer)
Error       — no se completó, con el motivo
```

## 9. Lo que la interfaz nunca decide por su cuenta

Todo lo de las secciones 4 y 5 se **revalida contra `AppCore`/SQLite en el
momento de confirmar** — nada de lo mostrado mientras se tecleaba se
considera una autorización reservada. Si algo cambió entre que se armó la
tarjeta de confirmación y el Enter (otro operador registró el mismo gafete,
alguien desactivó al contratista), la confirmación puede fallar con un error
nuevo aunque la tarjeta mostrada segundos antes se veía en verde. Esto es
intencional y es información útil para el diseño de la mutación de esa
tarjeta: el resultado mostrado es siempre "la mejor lectura disponible
ahora", no una promesa.

## 10. Los "estados de pantalla" tal como existen hoy

No hay pantallas nombradas — el contexto se deriva del input — pero estas
son las formas concretas que puede tomar el área contextual hoy
(`ContextState`), útiles como inventario de escenas de partida:

```text
Inicio                — vacío: título, total de gente adentro, comandos disponibles
Coincidencias          — resultados de búsqueda de contratistas (ingreso/editar/texto libre)
CoincidenciasActivos   — resultados de búsqueda de ingresos activos (/salida)
ResumenIngreso         — tarjeta de validación antes de confirmar un ingreso
ResumenSalida          — tarjeta de confirmación de una salida
TablaActivos           — /activos: tabla completa de quién está adentro
FichaContratista       — un contratista de la búsqueda de texto libre, sin acción de ingreso
ConfirmarCerrarSesion  — tarjeta de confirmación de /cerrarsesion
NuevoContratista       — tarjeta de entrada al alta (antes de abrir el formulario)
Ayuda                  — lista de comandos
MensajeError           — comando desconocido, parámetro inválido, o error de consulta
```

A esto se suma, como capa aparte, el formulario (sección 6) con sus propias
tres subfases: Editando → EligiendoEmpresa (opcional) → Resumen.

## Notas para el diseño de escenas

- La sección 7 es la que más importa para decidir dónde va cada mutación:
  cualquier transición **antes** de una de esas tres confirmaciones es
  reversible sin costo — el diseño puede (y probablemente debería) tratarlas
  con más libertad de movimiento que la confirmación en sí.
- El inventario de la sección 10 ya está agrupado de forma natural en los
  patrones que se venían discutiendo: `Coincidencias`/`CoincidenciasActivos`
  → BUSCAR/SELECCIONAR; el formulario completo → FORMULARIO;
  `ResumenIngreso`/`ResumenSalida`/Resumen del formulario → OPERACIÓN +
  CONFIRMACIÓN; el feedback transitorio (sección 8) → FEEDBACK.
- La sección 9 es la razón por la que una tarjeta de confirmación necesita
  poder mostrar visualmente "esto puede haber cambiado" si la mutación
  tarda o si se diseña alguna forma de revalidación en vivo — no es un caso
  hoy cableado en el código, pero el dominio lo permite y quizás valga la
  pena contemplarlo al diseñar esa escena.
