# Auditoría funcional y visual de la TUI (2026-08-20)

Evaluación en modo solo lectura de la interfaz terminal de BRISAS CLI. La revisión se
realizó a partir de los layouts de Ratatui, estados de interacción, navegación,
componentes compartidos y pruebas visuales existentes.

El análisis se orienta a tres aspectos:

1. Funcionalidad para el operador.
2. Mantenibilidad del sistema visual.
3. Calidad y seguridad de la experiencia de uso.

## Resumen ejecutivo

La TUI está bien construida y es claramente superior a una interfaz terminal improvisada.
Tiene identidad visual, navegación predecible, adaptación al tamaño disponible, buenos
estados vacíos y una orientación productiva al teclado. No se recomienda rediseñarla
desde cero.

La principal oportunidad está en convertir los patrones visuales repetidos en un sistema
de componentes más completo. También conviene reforzar la accesibilidad, la duración de
los mensajes y la claridad con la que se presentan filtros, advertencias y consecuencias
de las operaciones.

Valoración aproximada:

| Área | Valoración |
|---|---:|
| Funcionalidad | 8/10 |
| Consistencia visual | 7.5/10 |
| Experiencia del operador | 7.5/10 |
| Mantenibilidad visual | 6.5/10 |
| Accesibilidad | 6.5/10 |
| Cobertura visual automatizada | 6/10 |

## Estado de implementación en `mejora/auditoria-ui-tui`

| Hallazgo | Estado | Implementación |
|---|---|---|
| 1 | Avance sustancial | Campos, opciones, separadores y layout maestro-detalle se centralizaron en `ui_kit`; quedan candidatos adicionales como estados vacíos y confirmaciones. |
| 2 | Avance sustancial | Tablas, campos y paneles enfocados incorporan marcadores textuales; las decisiones críticas incluyen etiquetas además del color. |
| 3 | Avance sustancial | Una matriz automatizada renderiza 12 pantallas en 5 tamaños y 2 temas (120 combinaciones); queda como mejora futura incorporar snapshots aprobados. |
| 4 | Completado | Navegar ya no descarta mensajes importantes y las recargas conservan confirmaciones de éxito. |
| 5 | Completado | El pie describe la acción contextual de Esc y se respeta la jerarquía modal, edición, filtro y navegación. |
| 6 | Completado | Las búsquedas muestran ejemplo, filtros interpretados, texto libre y términos no reconocidos. |
| 7 | Completado | Activos separa la decisión histórica, condición actual, motivo, nivel y acción sugerida. |
| 8 | Parcial | Las confirmaciones sensibles explican mejor la consecuencia; aún conviene consolidarlas en un componente único con todo el contexto del objeto. |
| 9 | Parcial | Los atajos globales sólo se muestran cuando aplican y F2/F7 quedaron uniformes; las letras locales siguen siendo propias de cada pantalla. |
| 10 | Completado | Tema, columnas de Activos y Contratistas, vista y columnas de Historial se guardan localmente. |
| 11 | Completado | Tamaños mínimos, breakpoint maestro-detalle y disposición adaptable se centralizaron. |

Validación automatizada de la rama: 556 pruebas y `cargo clippy --all-targets -- -D warnings`.

## Fortalezas verificadas

### Separación clara por pantalla

Cada módulo separa estado, renderizado y pruebas. `app.rs` coordina los efectos con el
núcleo. Esto permite probar la navegación sin depender de la terminal ni de SQLite.

Referencias:

- `src/tui/activos/state.rs`
- `src/tui/activos/render.rs`
- `src/tui/activos/tests.rs`
- `src/tui/app.rs`

### Lenguaje visual semántico

`Theme` define fondo, texto, acento, éxito, advertencia, peligro, bordes y selección. Las
pantallas expresan intención semántica en lugar de elegir colores directamente.

Referencia:

- `src/tui/ui_kit/theme.rs:4-104`

### Marco y contexto consistentes

`ScreenShell` unifica producto, pantalla actual, usuario/rol, reloj, estado, comandos y
ayuda contextual. Además, distribuye los comandos en varias líneas cuando no caben.

Referencia:

- `src/tui/ui_kit/shell.rs:44-225`

### Diseño adaptable

Las pantallas principales cambian de tabla con panel lateral a una composición vertical
cuando el ancho es menor a 100 columnas. En tamaños inseguros muestran un aviso explícito
en lugar de renderizar contenido roto.

Referencias:

- `src/tui/activos/render.rs:18-20`
- `src/tui/nuevo_ingreso/render.rs:17-19`
- `src/tui/ui_kit/shell.rs:252-273`

### Interacción productiva por teclado

Las convenciones principales son coherentes: flechas para navegar, Enter para la acción
primaria, Esc para cancelar/volver, `/` para buscar, F1 para ayuda, F2 para salida rápida,
F4 para columnas y F7 para tema.

Referencia:

- `src/tui/ui_kit/keyboard.rs`

### Buenos estados vacíos y confirmaciones

Las pantallas explican cuándo se debe escribir para buscar, cuándo no hay resultados y
cuándo una operación necesita confirmación. Esto reduce acciones accidentales en un
sistema operativo sensible.

## Hallazgos y oportunidades

### 1. [ ] Media-alta — El sistema visual contiene componentes duplicados

Varias pantallas implementan por separado funciones casi idénticas para campos, opciones,
separadores y composiciones maestro-detalle.

Ejemplos:

- `src/tui/activos/render.rs:160-203`
- `src/tui/contratistas/render.rs:204-266`
- `src/tui/usuarios/render.rs:211-262`
- `src/tui/nuevo_ingreso/render.rs:155-218`
- `src/tui/empresas/render.rs:160-202`

Impacto: correcciones visuales deben repetirse, las pantallas pueden separarse lentamente
en comportamiento y aumenta el costo de pruebas.

Acción propuesta: ampliar `ui_kit` con componentes reutilizables:

- `FormField`.
- `ChoiceField`.
- `MasterDetailLayout` adaptable.
- Separadores comunes.
- `EmptyState`.
- `ConfirmationView`.
- `StatusMessage`.

Criterio de cierre: las pantallas consumen primitivas comunes sin perder la separación
entre estado y renderizado, y una mejora visual transversal requiere modificar un único
componente.

### 2. [ ] Media — Foco, selección y severidad dependen demasiado del color

La selección suele usar fondo coloreado y el panel enfocado cambia el color del borde.
Algunas vistas también emplean `>` o `[x]`, pero la señal no es uniforme.

Referencias:

- `src/tui/ui_kit/theme.rs:82-87`
- `src/tui/ui_kit/shell.rs:275-297`
- `src/tui/activos/render.rs:333-354`

Impacto: terminales con color limitado, configuraciones de contraste particulares o
usuarios con dificultad para distinguir colores pueden perder información importante.

Acción propuesta:

- Acompañar siempre el color con `>`, `[x]`, `!` o una etiqueta textual.
- Marcar el foco de panel de forma consistente.
- Reservar color de peligro para errores y acciones destructivas.
- Verificar ambos temas bajo contraste reducido y una paleta de 16 colores.

### 3. [ ] Media — Cobertura visual automatizada insuficiente

Existen buenas pruebas de estados y algunas verificaciones mediante `TestBackend`, pero no
hay una matriz sistemática de snapshots para todas las pantallas y tamaños relevantes.

Referencias:

- `src/tui/ui_kit/shell.rs:301-416`
- `src/tui/salida_rapida/tests.rs:159-201`
- `src/tui/historial/tests.rs:395-500`
- `src/tui/menu_principal/tests.rs:11-44`

Riesgos no cubiertos completamente:

- Texto cortado o superpuesto.
- Comandos que no caben.
- Campos que desaparecen en terminales bajas.
- Cursor fuera del campo visible.
- Diferencias accidentales entre layout lateral y apilado.
- Modales, errores o ayuda expandida que desplazan contenido.

Acción propuesta: probar cada pantalla y estado relevante en 60×22, 80×24, 99×30,
100×30 y 140×40. Comparar el buffer completo o snapshots normalizados.

### 4. [ ] Media — Mensajes importantes pueden desaparecer demasiado pronto

Algunas pantallas limpian el mensaje al recibir cualquier tecla. En Activos, por ejemplo,
el modo normal elimina `mensaje` antes de interpretar la acción.

Referencia:

- `src/tui/activos/state.rs:242-267`

Impacto: un error o confirmación puede desaparecer cuando el operador sólo intenta mover
la selección.

Acción propuesta:

- Mantener errores hasta una nueva operación relevante o descarte explícito.
- Mantener éxitos durante un intervalo o hasta iniciar otra operación.
- No borrar mensajes por navegación vertical.
- Diferenciar mensajes persistentes de información transitoria.

### 5. [ ] Media — Esc tiene demasiados significados contextuales

Según la pantalla y el modo, Esc puede cancelar, cerrar un selector, limpiar una búsqueda,
volver o salir. Etiquetas como `Limpiar/Volver` requieren que el usuario recuerde el estado
actual.

Ejemplos:

- `src/tui/activos/render.rs:23-40`
- `src/tui/activos/state.rs:242-303`
- `src/tui/nuevo_ingreso/render.rs:21-32`

Acción propuesta: formalizar y probar una jerarquía global:

1. Cerrar modal o selector.
2. Cancelar edición.
3. Limpiar filtro activo.
4. Volver a la pantalla anterior.
5. Salir de la aplicación sólo desde la raíz y con confirmación.

El pie de comandos debe describir exactamente qué ocurrirá en el estado actual.

### 6. [ ] Media — La búsqueda avanzada es potente pero poco descubrible

La sintaxis `clave:valor`, listas y negaciones favorece al operador frecuente. F1 muestra
ayuda, pero un usuario nuevo no ve claramente qué filtros fueron interpretados ni cuándo
una clave inválida terminó tratándose como texto libre.

Referencias:

- `src/tui/ui_kit/query_lang.rs`
- `src/tui/activos/state.rs:18-96`
- `src/tui/ui_kit/shell.rs:106-131`

Acción propuesta:

- Mostrar un ejemplo breve cuando la búsqueda esté vacía.
- Conservar F1 para la referencia completa.
- Mostrar una línea con filtros activos ya interpretados.
- Informar de claves no reconocidas o valores inválidos.
- Diferenciar visualmente filtros estructurados y texto libre.

### 7. [ ] Media — Las condiciones de acceso necesitan mayor precisión visual

En la tabla de activos, los resultados distintos de `Permitido` agregan `!`. El panel de
detalle indica genéricamente que la condición actual requiere atención, pero no siempre
expone con suficiente jerarquía qué cambió o qué debe hacer el operador.

Referencias:

- `src/tui/activos/render.rs:272-290`
- `src/tui/activos/render.rs:294-326`

Acción propuesta: presentar separadamente:

- Decisión tomada al ingresar.
- Condición evaluada actualmente.
- Motivo concreto.
- Nivel: permitido, advertencia o acción requerida.
- Acción sugerida al operador.

No depender únicamente de `!` o del color de advertencia.

### 8. [ ] Media-baja — Los paneles de confirmación pueden aportar más contexto

Para una acción sensible, el operador debería poder confirmar inequívocamente el objeto y
la consecuencia. En una salida conviene destacar nombre, empresa, gafete y hora de
ingreso, además del nombre seleccionado.

Acción propuesta: crear un componente de confirmación que incluya:

- Acción en lenguaje directo.
- Identidad del objeto afectado.
- Datos necesarios para evitar confusión.
- Consecuencia.
- Atajos de confirmar y cancelar.
- Estilo especial para acciones destructivas o irreversibles.

### 9. [ ] Media-baja — Los atajos por letras requieren mayor consistencia global

Letras como `A`, `C`, `E`, `N`, `P` y `R` cambian de significado entre pantallas. Esto es
aceptable localmente, pero aumenta la carga de aprendizaje.

Acción propuesta:

- Reservar F1, F2, F4, F7, Esc y navegación como comandos globales documentados.
- Mantener la misma letra para acciones equivalentes.
- Mostrar sólo comandos válidos en el modo actual.
- Añadir una referencia global de teclado en la ayuda.

### 10. [ ] Baja — Preferencias visuales aparentemente no persistentes

El tema y las columnas visibles parecen mantenerse únicamente en memoria. Un operador
puede tener que reconfigurar su vista después de cada inicio.

Referencias:

- `src/tui/app.rs`
- `src/tui/activos/state.rs:152-180`
- `src/tui/contratistas/state.rs`

Acción propuesta: persistir localmente o por usuario:

- Tema.
- Columnas visibles y orden.
- Vista preferida del historial.
- Otras preferencias que no afecten reglas de negocio.

### 11. [ ] Baja — Umbrales y medidas visuales están distribuidos por pantalla

Los valores 60×22 y el umbral lateral de 100 columnas se repiten. Aunque ahora son
coherentes, podrían divergir conforme evolucione la aplicación.

Referencias:

- `src/tui/activos/render.rs:18-20`
- `src/tui/contratistas/render.rs:21-23`
- `src/tui/usuarios/render.rs:19-21`
- `src/tui/historial/render.rs:19-21`
- `src/tui/nuevo_ingreso/render.rs:17-19`

Acción propuesta: centralizar breakpoints y documentar qué garantiza cada categoría de
tamaño, permitiendo excepciones explícitas para formularios especiales.

## Ruta de mejora sugerida

### Fase 1 — Consistencia sin cambiar flujos

1. Extraer campos, opciones, separadores, estados vacíos y layout maestro-detalle a
   `ui_kit`.
2. Centralizar breakpoints y formateadores visuales.
3. Añadir señales no cromáticas de foco y severidad.
4. Aplicar snapshots visuales antes de refactorizar para proteger el aspecto actual.

### Fase 2 — Claridad operativa

1. Mejorar persistencia de mensajes.
2. Unificar la semántica de Esc.
3. Hacer visibles los filtros interpretados.
4. Mostrar razones y acciones de acceso con mayor jerarquía.
5. Estandarizar confirmaciones sensibles.

### Fase 3 — Personalización y validación real

1. Persistir tema y columnas.
2. Ejecutar pruebas de uso con operadores reales.
3. Medir tiempo y errores en los flujos de entrada, salida rápida y búsqueda.
4. Ajustar textos y atajos a partir de evidencia observada.

## Pruebas de experiencia recomendadas

- Registrar una entrada normal sin ayuda externa.
- Registrar una entrada con advertencia de PRAIND.
- Encontrar y cerrar rápidamente un ingreso activo.
- Recuperarse de una búsqueda sin resultados.
- Distinguir decisión histórica de condición actual.
- Completar las tareas anteriores usando sólo el pie de comandos.
- Repetirlas en 60×22, 80×24 y 140×40.
- Repetirlas con ambos temas y con soporte de color limitado.

Indicadores útiles:

- Tiempo hasta completar cada tarea.
- Pulsaciones incorrectas.
- Cancelaciones accidentales.
- Uso de F1.
- Consultas `clave:valor` mal interpretadas.
- Confirmaciones rechazadas por falta de certeza.

## Criterio final

La interfaz actual es funcional, coherente y suficientemente buena para evolucionar de
forma incremental. Su principal riesgo no es estético: es que la duplicación de renders,
los mensajes fugaces y las señales demasiado sutiles produzcan divergencias o errores
operativos a medida que crece la aplicación.

La prioridad debe ser consolidar el sistema visual, mejorar la claridad de las decisiones
y cubrir el renderizado mediante snapshots. Estas mejoras pueden realizarse sin cambiar
la arquitectura general ni rediseñar la aplicación completa.
