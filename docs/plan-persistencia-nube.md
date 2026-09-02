# Persistencia en la nube — definiciones pendientes (borrador, sin código todavía)

> Documento de continuidad para retomar esta conversación en otra sesión
> (VS Code). Recoge lo que ya se discutió y, sobre todo, la pregunta de
> arquitectura que quedó sin resolver antes de escribir una sola línea de
> código. Nada de lo de acá está implementado.

## Por qué existe este documento

Este tema **no es solo de la app móvil** — la app de escritorio (Tauri,
`desktop/`) es la que existe primero y probablemente también necesite
esto. Antes de diseñar nada hace falta entender cómo se reparte el
trabajo entre el núcleo de Rust y cada plataforma, porque esa respuesta
decide dónde vive el código nuevo.

## Contexto de la conversación (resumen)

- Intento anterior con una app de escritorio distinta usando Supabase:
  "el infierno en la tierra" — no se logró lo buscado. Motivo probable
  (a confirmar, no descartar Supabase por completo): Supabase empuja a un
  modelo de base compartida en vivo con sincronización/tiempo real, que
  no es lo que este proyecto necesita.
- Modelo que sí se quiere:
  - **La base local manda siempre.** Si la nube no responde, la app
    sigue funcionando local sin enterarse — mismo criterio que ya usa
    hoy la app móvil (todo contra SQLite local, sin validar en vivo
    contra nada externo).
  - **Dos categorías de datos, con urgencia distinta:**
    - **Espejo** (contratistas, empresas, usuarios): cambian poco: tienen
      que existir igual en la base local y en la nube, pero no hace
      falta que sea instantáneo.
    - **Cola** (ingresos/salidas): cambian todo el tiempo, pero pueden
      esperar en una cola y mandarse cuando haya conexión — no hace
      falta en vivo.
  - **Ya no se piensa como "varios dispositivos sincronizando entre
    sí"** (eso es lo que hacía difícil el intento anterior: conflictos
    entre escritores). Se piensa como **"cada lugar (garita, sitio)
    dueño de su propia base local, que empuja lo suyo hacia un mismo
    receptor en la nube, sin necesitar leer lo de los demás"** — un
    modelo de reporte de una sola vía (dispositivo → nube), no de
    sincronización bidireccional. Sin esa lectura de vuelta, no hay
    conflictos que resolver: dos sitios nunca pisan el dato del otro.
  - Consulta externa a esos datos agregados (para ver todos los sitios
    juntos) sería una capa aparte, separada de los dispositivos — ni el
    teléfono ni la PC necesitan saber que existe.
- Ya se habló de tres formas posibles de armar el receptor en la nube
  (servidor propio a medida / Postgres administrado usado sólo como
  destino de inserciones / reportes periódicos por archivo). **Se
  eligió la primera: un receptor propio, chico, hecho a medida del
  problema** — no un backend genérico tipo Supabase.
- **Pendiente sin responder todavía:** si lo de "cada lugar con su
  propia base, compartido bajo consulta externa" (multi-sede) es un
  requisito de ahora o algo para más adelante. Se preguntó y la
  respuesta quedó sin dar — retomar esto antes de diseñar el receptor,
  porque cambia su forma (una sola sede vs. varias sedes reportando al
  mismo lugar).
- Relacionado: `docs/plan-app-movil.md`, sección "Abierto", ya tenía
  anotado desde el plan original de la app móvil que el mecanismo exacto
  de "reportar a la PC" quedaba sin definir — este documento retoma esa
  misma pregunta con más contexto, ahora que se habla de nube y no sólo
  de la PC de la garita.

## La pregunta central, tal cual se planteó

> ¿El núcleo de Rust se encarga de todo y sólo le manda las peticiones al
> kernel del teléfono, y éste las ejecuta? ¿O hay que programar algo
> específico en el teléfono, distinto de lo que hace falta en la PC?

## Lo que ya se sabe con certeza (verificado en el código, no supuesto)

- `control_acceso` (raíz del repo) es el núcleo de negocio puro — sin
  nada específico de plataforma adentro.
- **Escritorio (`desktop/src-tauri/Cargo.toml`)** depende de
  `control_acceso` como librería Rust directa:
  `control_acceso = { path = "../..", ... }`. Tauri es Rust — no hay
  ningún cruce de lenguaje entre la app de escritorio y el núcleo, es
  todo el mismo binario compilado junto.
- **Móvil (`mobile/rust-core/Cargo.toml`)** depende de `control_acceso`
  de la misma forma: `control_acceso = { path = "../..", ... }`. La
  diferencia no está en cómo se usa el núcleo, sino en que **Kotlin/JVM
  no puede llamar funciones Rust directamente** — hace falta cruzar esa
  frontera de lenguaje, y para eso existe `mobile/rust-core` como puente
  (uniffi genera los bindings Kotlin, JNA hace la llamada FFI real). Esa
  capa de traducción es exclusiva de Android — el escritorio no la
  necesita porque no hay ningún idioma que cruzar ahí.
- **Hoy no existe ni un solo cliente HTTP en todo el proyecto** —
  revisado `Cargo.toml` de la raíz, de `desktop/src-tauri` y de
  `mobile/rust-core`: ninguno tiene `reqwest`, `hyper`, `ureq` ni
  ninguna otra dependencia de red. Esto se diseña desde cero, no se
  está adaptando nada existente.

## La respuesta preliminar (a confirmar juntos, no es definitiva)

Con esas dos piezas, la lectura más probable es:

- **La lógica de red (armar el paquete, mandarlo, reintentar) puede
  vivir en Rust compartido**, igual que toda la lógica de negocio hasta
  ahora — mismo criterio que ya sostiene el proyecto ("el núcleo se
  reutiliza tal cual, nunca se traduce a otro lenguaje"). Una librería
  Rust para hacer HTTP (ej. `reqwest` o similar) compila igual para
  escritorio y para Android — no habría que escribir esa parte dos
  veces.
- **Lo que sí sería distinto por plataforma es el disparador**: quién
  decide *cuándo* intentar vaciar la cola. Ya se confirmó que la app
  móvil no tiene ningún servicio en segundo plano ni `WorkManager` — si
  se quiere un envío automático (no sólo "cuando el guardia tiene la
  app abierta"), eso sí requiere algo específico de Android
  (`WorkManager` es la forma estándar de programar trabajo diferido/en
  background ahí). El escritorio tendría su propio mecanismo equivalente
  (más simple, ya que no tiene las mismas restricciones de batería/OS
  que Android). Esa pieza puntual — el disparador — es la que
  probablemente sí haga falta programar distinto en cada lado.

Esto es una lectura de arquitectura, no una decisión tomada — falta
confirmarla juntos antes de escribir nada.

## Decisiones tomadas (sesión 2026-09-02)

- **Multi-sede: confirmado, es un hecho desde ya** — no es "más adelante".
  Ya hay al menos un sitio (Brisas) con dos dispositivos (PC + celular);
  se suman más sitios (Cartago, Belén, etc.) sin duda.
- **Lectura Rust compartido vs. específico de plataforma: confirmada**,
  verificado revisando el código de `mobile/` (no hay ni un cliente HTTP,
  ni tabla de cola, ni `WorkManager` todavía — se parte de cero pero sin
  ningún obstáculo arquitectónico). La lógica de red y de la cola vive en
  Rust compartido; lo específico por plataforma es solo el disparador.
- **Espejo y cola se unifican en un solo mecanismo**: una bandeja de
  salida (patrón *outbox*) con un campo de prioridad, en vez de dos
  sistemas separados. Menos código, menos cosas que romper.
- **Identidad de dispositivo**: cada dispositivo (no cada sitio) tiene su
  propia credencial — un secreto aleatorio de alta entropía, *no*
  derivado de datos públicos (nombre/dirección), generado desde el panel
  de administración, mostrado una sola vez, guardado hasheado en el
  receptor (igual que una contraseña). La metadata descriptiva (sitio,
  dirección, tipo de dispositivo) se guarda aparte, sin relación con el
  secreto. Permite revocar un dispositivo puntual (ej. celular
  perdido/robado) sin afectar a los demás.
- **Una sola pieza de software**: el receptor (recibe datos de los
  dispositivos) y el panel de administración/auditoría (alta de
  dispositivos + consulta cruzada entre sitios) son la misma app, sin
  duplicar la base de credenciales — quien audita ya tiene acceso
  privilegiado, no hace falta separarlo.
- **Entre sitios distintos**: sigue sin haber lectura cruzada — Cartago
  nunca sabe que Brisas existe. Lo hace cumplir automáticamente la
  credencial de cada dispositivo (queda atada a su sitio al generarse),
  vía Row Level Security en la base.
- **Dentro de un mismo sitio (PC + celular)**: sí necesitan verse entre
  sí — ya no es "una sola vía" a ese nivel. Un ingreso creado en la PC
  tiene que poder cerrarse (registrar salida) desde el celular del mismo
  sitio, y viceversa. Esto ya no es "fuente de verdad central en la PC"
  como se pensaba en `docs/plan-app-movil.md` — PC y celular son pares
  simétricos del mismo sitio.
- **Conflictos** (los dos dispositivos intentan cerrar el mismo ingreso
  casi a la vez): control de concurrencia optimista, "el primero que le
  llega a la nube gana"; el segundo recibe un rechazo con el motivo
  ("ya se cerró desde [PC/celular] a las hh:mm") y lo refleja localmente.
- **Receptor elegido: Supabase (plan Pro, ~US$25/mes)**, no receptor
  100% propio en VPS ni PC con túnel — se descarta mantener
  infraestructura propia porque no hay quien la administre 24/7. Se
  usa solo como Postgres alojado + API REST automática (PostgREST) +
  Row Level Security para separar sitios + Realtime (Postgres Changes,
  ya incluido en el plan Pro, sin costo extra a esta escala) para el
  relevo en vivo PC↔celular del mismo sitio. **Deliberadamente sin
  usar Realtime para el panel de auditoría entre sitios** — ahí alcanza
  con consulta bajo demanda, no hace falta instantáneo.
  - **Nota importante, no cosmética**: esto es una excepción a la regla
    de "todo vive en Rust, nunca se traduce" que sostiene el resto del
    proyecto — la lógica de conflicto/outbox del lado receptor vive en
    parte en Postgres (funciones/políticas RLS), no en Rust.
- **Notificaciones push nativas (Firebase Cloud Messaging, aviso aunque
  la app esté cerrada): descartadas por ahora, sin presupuesto** — queda
  anotado para reconsiderar más adelante. Por ahora alcanza con que la
  pantalla se actualice sola mientras la app está abierta (ya cubierto
  por Realtime).

## Temas para la próxima sesión, en orden

1. Diseñar el esquema concreto de la bandeja de salida (outbox) en
   SQLite local: columnas, estados (pendiente/enviado/fallido), qué pasa
   si un envío falla a medias.
2. Diseñar las tablas en Supabase/Postgres y las políticas de RLS que
   separan sitios (y permiten que PC y celular del mismo sitio se vean
   entre sí).
3. Definir la forma exacta del secreto de dispositivo (cómo se genera,
   formato del hash, cómo se pega/registra en la app de cada
   dispositivo) y el formulario de alta en el panel de administración.
4. Implementar el rechazo por conflicto (optimistic concurrency) del
   lado del receptor — la regla "primero en llegar gana" concreta como
   función/policy de Postgres.
5. Decidir el disparador de sincronización fuera del caso "mismo sitio en
   vivo": ¿cómo y cuándo se vacía la bandeja de salida hacia Supabase en
   general (al recuperar conexión, periódico, manual)? En Android, si
   más adelante se quiere reintento en segundo plano real, hace falta
   `WorkManager` (hoy inexistente) — queda diferido, no bloquea el resto.
