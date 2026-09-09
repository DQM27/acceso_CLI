# Control de visitas — plan (borrador, sin código todavía)

> Documento de continuidad para retomar esta conversación en otra sesión.
> Nace de una consulta del usuario sobre un proyecto nuevo (control de
> acceso para visitas agendadas, distinto de contratistas) y si convenía
> reutilizar el núcleo Rust compartido. Nada de esto está implementado
> todavía — es la base para decidir, sesión por sesión, igual que
> `plan-persistencia-nube.md`, `plan-sesion-unica-dispositivos.md` y
> `plan-panel-administrativo-web.md`.

## Por qué existe este documento

Sesión de planificación (sin tocar código) sobre cómo agregar "visitas"
(y a futuro "proveedores") al sistema. Contexto de negocio dado por el
usuario: hoy existe un mecanismo de agendamiento con QR por correo que
el usuario considera poco idóneo; la alternativa acordada es cédula en
garita, igual que contratistas.

## Decisión de fondo: reutilizar el núcleo, no la tabla de contratistas

Contratista y Visita comparten infraestructura (núcleo Rust
`control_acceso`, `usuarios`, sincronización multi-dispositivo,
gafetes, dispositivos, `sitios`), pero **no comparten modelo de datos**
-- decisión explícita del usuario: "obviamente no vamos a mezclar
entidades eso sería absurdo". `registro_ingresos` está demasiado atado
a reglas específicas de contratista (PRAIND, `tipo_ingreso`
PRAIND/IN_HOUSE/POR_CORREO/SWAT, `es_personal_ruta`,
`fecha_vencimiento_praind`) como para reutilizarlo tal cual.

Mecanismos que SÍ se comparten sin cambios:
- El motor de sincronización (drenar cola, recibir del sitio, watermark
  incremental por marca de agua -- mismo patrón que ya usan
  contratistas/empresas/usuarios/gafetes/historial).
- `usuarios` (guardias/administradores de sitio) -- sin tocar.
- `sitios` -- se reutiliza tal cual como FK, no se duplica.
- El catálogo físico de `gafetes` (ver más abajo).

## Modelo de datos: patrón header-detail (estándar de industria,
confirmado por investigación -- ver Referencias)

Una **Cita** no es un ingreso -- es una **autorización con vigencia**:
válida entre una fecha y otra, dentro de la cual la persona puede
entrar y salir del sitio varias veces. Estructuralmente es más parecida
a un `Contratista` (identidad que genera muchos ingresos) que a un
ingreso suelto, salvo que tiene fecha de vencimiento y la crea otra
persona (el anfitrión), no el guardia.

Cuatro tablas nuevas, no dos:

1. **`citas`** (cabecera / la autorización)
   - motivo de la visita
   - fecha desde, fecha hasta (rango de vigencia)
   - quién la creó (anfitrión)
   - estado: VIGENTE / VENCIDA / CANCELADA

2. **`cita_sitios`** (puente muchos-a-muchos)
   - `cita_id`, `sitio_id`
   - Necesaria porque una cita puede aplicar a **varios sitios a la
     vez** (caso "tour": quien agenda marca todos los sitios donde la
     persona se va a presentar en esa fecha) -- decisión explícita del
     usuario. Si la cita es sólo para Brisas, sólo los guardias de
     Brisas la ven; si incluye Cartago también, los de Cartago también.
   - **Consecuencia técnica marcada para más adelante**: la RLS y el
     filtro de sincronización de `ingresos`/`gafetes`/`dispositivos`
     hoy comparan `sitio_id = mi sitio` directo. Para citas hace falta
     un `EXISTS` contra esta tabla puente en vez de una igualdad --
     patrón nuevo, no existe todavía en el resto del sistema.

3. **`cita_visitantes`** (detalle -- una fila por persona del grupo)
   - cédula, nombre, empresa, placa de vehículo (nullable)
   - Una cita puede agendarse para varias personas a la vez (una
     reunión con varios visitantes) -- decisión explícita del usuario:
     "se puede agendar una cita múltiple, solo va añadiendo campos".
     Motivo/fecha/anfitrión/sitios son compartidos por todo el grupo
     (viven en `citas`); cédula/nombre/empresa/placa son por persona.

4. **`movimientos_visita`** (el cruce real en garita -- equivalente a
   `registro_ingresos`, pero sin ningún campo de PRAIND/SWAT)
   - referencia a QUÉ `cita_visitante` lo autoriza (no a la cita
     entera -- cada persona del grupo entra y sale por su cuenta)
   - `sitio_id` (en cuál de los sitios autorizados se presentó; una
     misma persona puede tener movimientos en más de un sitio si la
     cita es multi-sitio, o varios movimientos en días distintos
     dentro de la vigencia)
   - gafete, hora entrada/salida, quién lo registró, campos de sync
     (uuid, dispositivo_entrada_id, etc. -- mismo patrón que
     `registro_ingresos`)

Check-in en garita: guardia escanea cédula → el sistema busca si hay un
`cita_visitante` con esa cédula, cuya `cita` esté VIGENTE hoy y incluya
este sitio en `cita_sitios` → si existe, se arma el movimiento (gafete,
etc.) igual que hoy se arma un ingreso de contratista, pero apuntando a
la cita en vez de al contratista.

**Cada tabla local nueva necesita su espejo en Supabase** -- no hay
atajo. La sincronización funciona porque cada tabla local que necesita
visibilidad cross-dispositivo tiene su reflejo en la nube (mismo
patrón que `contratistas`↔`contratistas`,
`registro_ingresos`↔`ingresos`). Una cita agendada desde la web tiene
que pasar por Supabase para que el guardia la vea en su PC o celular.

## Gafetes: qué se comparte y qué no

**Decidido (corrige un supuesto anterior de este documento): los
gafetes NO son un pool físico único.** Son físicamente distintos por
tipo de persona -- verde para contratista, rojo para visita, ámbar para
proveedor -- y **repiten la misma numeración entre colores** (el "7
verde" y el "7 rojo" son objetos distintos que coexisten). Consecuencia
directa en el esquema: la unicidad de `gafetes` pasa de `numero` solo a
el par `(numero, tipo)`, y la tabla gana una columna `tipo` (CHECK
`CONTRATISTA`/`VISITA`/`PROVEEDOR`). Esto no mezcla datos de personas
entre tipos -- es la etiqueta de a qué pool físico pertenece esa fila
del catálogo, sigue siendo una sola tabla (no tres), simplemente ya no
es "un solo catálogo sin distinción" como se asumió al principio.

**El "deudor" (quién debe el gafete si se pierde) sigue el mismo
criterio ya decidido, ahora reforzado por `tipo`**: tres columnas
nullable, cada una con su FK real (`contratista_deudor_id`,
`cita_visitante_deudor_id`, `proveedor_deudor_id` cuando exista), con
un `CHECK` que exige exactamente una seteada -- pero ahora ese `CHECK`
puede apoyarse en `tipo` para exigir que sólo la columna del color
correspondiente pueda tener valor (un gafete `tipo = 'VISITA'`
estructuralmente no puede tener `contratista_deudor_id` seteado, lo
impide la base, no queda librado a que nadie se equivoque a mano). Se
prefirió este camino sobre un campo genérico `deudor_tipo` + `deudor_id`
sin FK porque este proyecto valora la integridad referencial real
(tablas STRICT, `CHECK`s por todos lados) y un campo así la perdería.

Los movimientos (`registro_ingresos` para verdes,
`movimientos_visita` para rojos) no necesitan repetir `tipo` -- cada
tabla ya está scopeada a un solo color por diseño, así que un
"gafete 7" ahí nunca se confunde con el de otro color.

## Identidad de anfitriones ("KOF")

El anfitrión (a quién viene a ver la visita, término interno de la
empresa: "KOF") **no es un `usuario`** de los que ya existen (esos son
guardias/admins de sitio, login offline, sincronizan completos a cada
dispositivo de garita). Es un actor nuevo: entra solo por web, con
conexión, agenda sus propias citas y (más adelante, no prioridad) recibe
aviso de sus visitantes.

**Reutiliza el mismo molde que `administradores_panel`** (ver
`supabase/migrations/20260905024834_crea_administradores_panel.sql`):
Google Auth resuelve la identidad, una tabla nueva `anfitriones`
(correo como clave, cédula si hace falta cruzarla, nombre, sitio(s)
donde puede agendar) decide autorización real, RLS "cada quien
lee/gestiona sólo su propia fila" (`auth.email() = correo`), alta
administrativa de la lista -- sin autoregistro, mismo criterio que
admins.

Tres identidades separadas conviviendo, cada una con su propio
mecanismo:
- `usuarios` → guardias/admins de sitio, login offline, apps de garita.
- `administradores_panel` → quien administra el panel web completo.
- `anfitriones` (nueva) → quien agenda y (a futuro) recibe aviso de sus
  visitas, en el subdominio nuevo.

## Cliente: web en subdominio propio, no una app móvil nueva

Recomendación dada y sin objeción del usuario: **no conviene una app
móvil nueva para quien agenda**. Agendar es una acción esporádica (no
un hábito diario que justifique instalar algo), no necesita cámara/OCR
ni nada nativo, y no necesita funcionar offline -- todo lo contrario
al motivo de ser de la app de garita. Una web alcanza y sobra.

Confirmado técnicamente: Cloudflare Workers soporta dominios/subdominios
custom sin fricción (mismo mecanismo que ya usa `panel-brisas`, y el
proyecto ya tiene `megabrisas.com` como dominio propio conectado a
Supabase Auth). Un subdominio nuevo (ej. `agendar.megabrisas.com`) es
un Worker nuevo + una ruta de dominio -- trabajo de configuración
cuando llegue el momento, no un obstáculo de arquitectura.

## Notificación al anfitrión -- explícitamente NO prioridad

Decisión del usuario: "la notificación no es prioridad, eso es más
estético que otra cosa, puede ir después". Dos caminos posibles cuando
se retome, sin decidir todavía:
- Correo (canal que ya existe, más simple).
- Realtime dentro de la página (ya hay infraestructura de presencia/
  Realtime por sitio para la sincronización -- se podría enganchar ahí
  para que aparezca sin recargar).

## Pendiente, fuera de alcance de esta sesión

- **Proveedores**: el otro actor mencionado como faltante. Se espera
  que siga un patrón parecido (cédula + algo que lo autorice + gafete),
  pero no se diseñó en detalle todavía -- probablemente reutilice
  `cita_sitios`/`movimientos_*` con ajustes, o necesite su propia
  cabecera si su ciclo de vida es distinto al de una cita con vigencia.
- **Notificación al anfitrión** (ver arriba).

## Preguntas abiertas

- ¿El campo `motivo` de la cita es obligatorio u opcional?
- ¿Quién puede cancelar una cita ya creada -- sólo el anfitrión que la
  creó, o también un administrador del sitio?
- Detalle de la mecánica de "vigencia": ¿una cita vencida se oculta
  sola, o hace falta un estado explícito que alguien setee (más allá
  de comparar fechas en la consulta)?

## Referencias

Investigación hecha para validar que el modelo (header-detail: cita +
detalle de visitantes + puente de sitios + movimientos) es el estándar
de la industria, no algo improvisado:

- [How to Design a Database for Event Management (GeeksforGeeks)](https://www.geeksforgeeks.org/dbms/how-to-design-a-database-for-event-management/)
- [Event Management Database Design Part 1 (Medium)](https://medium.com/@arpita_deb/event-management-database-design-part-1-5239620410c1)
- [A definitive guide to visitor management (Eptura)](https://eptura.com/resource/visitor-management-guide/)
- [Visitor Management Process: Complete 2026 Guide (Elia)](https://www.elia.io/blog/visitor-management-process)
- [Visitor-to-Access in One Flow: Best Practices (FacilityOS)](https://www.facilityos.com/blog/visitor-to-access-in-one-flow)
