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

## Temas para la próxima sesión, en orden

1. Resolver la pregunta pendiente: multi-sede ¿ahora o después?
2. Confirmar (o corregir) la lectura preliminar de arriba: qué va en Rust
   compartido y qué es específico de cada plataforma.
3. Decidir el disparador de sincronización en cada plataforma (¿manual,
   automático al recuperar conexión, periódico?).
4. Diseñar el esquema de la cola local (tabla nueva, estados
   pendiente/enviado/fallido, qué pasa si un envío falla a medias).
5. Diseñar el receptor (ya elegido: propio, a medida) — dónde vive
   (PC con túnel de Cloudflare / VPS), forma del paquete que recibe,
   cómo identifica de qué sitio/dispositivo viene cada dato.
6. Confirmar si la PC de escritorio también manda su propia cola al
   mismo receptor, o sigue siendo la "fuente de verdad" central como se
   pensó en el plan original de la app móvil (`docs/plan-app-movil.md`) —
   esto cambia bastante el diseño y quedó sin decir explícitamente en
   esta conversación.
