# Plan: autenticación de usuarios globales contra Supabase Auth

## El problema que esto resuelve

Hoy, un usuario global (Administrador/Operador, sincronizado a todos los sitios vía
`usuarios` en Supabase) llega a cada dispositivo con el centinela `SIN_PASSWORD_LOCAL`
(`src/services/password.rs`). El primer login de esa cédula en un dispositivo puntual la
manda directo a `FijarPasswordInicial.tsx`, que fija la contraseña **sin verificar nada
más que la cédula** (`src/application/autenticacion.rs::fijar_password_inicial`, línea
74-79 lo dice explícito: "No exige conocer una contraseña anterior... sólo la cédula").

La cédula no es secreta -- va impresa en gafetes que esta misma app gestiona. Cualquiera
que sepa la cédula de un Administrador que todavía no inició sesión en un dispositivo
puntual puede reclamar esa cuenta ahí: escalada de privilegios con solo un número
público.

Además, los oficiales rotan entre varias unidades operativas -- necesitan poder cubrir
turno en cualquier sitio sin fricción, así que la solución no puede ser "un código de un
solo uso por dispositivo" (eso reintroduce fricción operativa en cada rotación).

## Diseño acordado

Reemplazar la autenticación local por dispositivo por autenticación siempre contra
Supabase, con un token de operación cacheado en memoria para seguir funcionando offline.

### Por qué Supabase Auth nativo, no una tabla propia

La tabla `usuarios` documenta a propósito que nunca guarda `password_hash` -- "la nube
distribuye quién existe y su rol/estado, nunca la contraseña" (comentario en
`20260905072034_crea_usuarios_globales.sql`). Meterle un hash ahí revertiría esa decisión,
porque esa tabla se lee ampliamente por cualquier dispositivo autenticado.

En vez de eso: **Supabase Auth (GoTrue)** guarda el hash en su propio store, separado de
`usuarios`. Se usa un email sintético interno (`<cedula>@brisas.local`, nunca se manda a
nadie) como identificador de login. `usuarios` sigue siendo la única fuente de verdad para
identidad/rol/estado; se le suma una columna `auth_user_id` que enlaza con
`auth.users.id`.

Esto además regala gratis el mecanismo de renovación: Supabase Auth ya entrega
`access_token` (corto) + `refresh_token` (para renovar sin pedir contraseña de nuevo) --
es exactamente lo que se iba a construir a mano con JWT firmado como hace
`device-auth/index.ts`, pero sin escribir cripto nueva.

### Ciclo de vida completo

**1. Alta de usuario (reemplaza el alta actual en `web/src/api/usuarios.ts::crearUsuario`)**

Un Admin desde el panel da de alta la cédula/nombre/rol como hoy, pero en vez de un
`insert` directo a `usuarios`, llama a un Edge Function nuevo (`admin-create-usuario`,
mismo patrón que `admin-provision-device`): éste usa `service_role` para:
  - Generar una contraseña temporal aleatoria de un solo uso, corta (no tan extensa como
    el secreto de dispositivo -- pensada para que un humano la transcriba/copie una vez).
  - Crear el usuario en Supabase Auth (`admin.createUser`) con esa contraseña y
    `user_metadata: { debe_cambiar_password: true }`.
  - Insertar la fila en `usuarios` con `auth_user_id` enlazado.
  - Devolver la contraseña temporal **una sola vez** en la respuesta -- el panel la
    muestra para copiar/mandar por WhatsApp o lo que sea, igual que ya hace con el secreto
    de dispositivo. No se guarda en ningún lado después de esto.

**2. Primer login real (en cualquier sitio)**

Cédula + contraseña temporal → `signInWithPassword` contra Supabase Auth. Si
`user_metadata.debe_cambiar_password` es `true`, la app fuerza la pantalla de cambiar
contraseña antes de dejar operar -- no se puede saltar. Ya no existe
`FijarPasswordInicial.tsx` como pantalla de "reclamo" -- el login es una sola pantalla
siempre, en todos los sitios.

**3. Cambiar contraseña (rutina)**

Pide contraseña actual + nueva. Revalida la actual con `signInWithPassword` (no confiar
en que la sesión esté abierta) y recién ahí `updateUser({ password })`, y limpia
`debe_cambiar_password` si estaba en `true`.

**4. Olvidó la contraseña**

Mismo actor que la creó: un Admin desde el panel dispara el mismo flujo de "generar
temporal" (mismo Edge Function, o uno de reset) -- `admin.updateUserById` con nueva
contraseña aleatoria + `debe_cambiar_password: true` de nuevo.

### Sesión y modo offline

- El `access_token`/`refresh_token` viven **solo en memoria** del proceso (Rust
  `GuiState` en desktop) -- nunca se persisten a disco.
- Mientras la app sigue corriendo y hay red, se renuevan en segundo plano aprovechando el
  mismo pulso de sync que ya existe (~2 min) -- sin pedirle nada al usuario.
- **Tope duro de 12h de presencia**: aunque el token técnico siga vivo, si pasaron 12h
  desde la última renovación exitosa contra Supabase, se fuerza a cerrar sesión. Esto lo
  aplica el cliente, no depende de la config de expiración del proyecto (que es global y
  afecta también al panel web).
- **Cerrar la app = perder la sesión**, siempre. El próximo arranque exige login real
  contra Supabase de nuevo, sin importar cuánto quedara del tope de 12h. No hay "resumir
  sesión" al reabrir.
- Verificación offline del token: se cachea el JWKS del proyecto (`/auth/v1/.well-known/jwks.json`)
  una vez, y se valida la firma localmente sin red en cada uso posterior dentro de la
  misma corrida del proceso.

### Trade-off aceptado

Con 12h de tope, si desactivan a alguien en el panel, sigue operando en un dispositivo que
ya tenía sesión vigente hasta que esa sesión se corte (máx. 12h) o hasta que intente
loguear de nuevo (ahí Supabase ya lo rechaza). Mismo principio que ya está aceptado para
"bloqueo hasta reconectar" en otras partes del sistema, acotado acá a un máximo conocido
en vez de indefinido.

## Alcance de esta iteración

- **Supabase**: migración (`auth_user_id` en `usuarios`) + Edge Functions de alta/reset
  con contraseña temporal de un solo uso.
- **Panel web** (`web/src/api/usuarios.ts`, `Usuarios.tsx`): cambiar `crearUsuario` para
  llamar al Edge Function nuevo y mostrar la contraseña temporal generada.
- **Núcleo Rust** (`src/services/`, `src/application/autenticacion.rs`): reemplazar
  verificación local de Argon2 por login contra Supabase + verificación de JWT offline.
  Elimina `SIN_PASSWORD_LOCAL`, `fijar_password_inicial`, y el `password_hash` local deja
  de ser la fuente de verdad.
- **Desktop** (`Login.tsx`, `FijarPasswordInicial.tsx` → una sola pantalla con el flujo de
  "cambiar contraseña obligatorio" integrado, comandos Tauri de sesión).
- **Fuera de alcance por ahora** (se anota como pendiente, no se toca hoy): Android
  (`LoginViewModel.kt`, `PantallaFijarPasswordInicial.kt`) -- ya estaba señalado como
  trabajo de auth pendiente en el núcleo mobile; se espeja después con el mismo patrón una
  vez validado en desktop.

## Progreso (para quien retome esto)

**Hecho y verificado:**

- **Supabase**: migración `20260911030000_enlaza_usuarios_con_supabase_auth.sql`
  (`auth_user_id` en `usuarios`) aplicada. Edge Functions `admin-create-usuario` y
  `admin-reset-password-usuario` desplegados y probados.
- **Panel web** (`web/src/api/usuarios.ts`, `Usuarios.tsx`): `crearUsuario` llama al Edge
  Function, muestra la contraseña temporal una sola vez, botón "Resetear contraseña" por
  fila. `tsc`/Vitest limpios.
- **Bloqueo de ROOT** (relacionado, no parte de este plan en sí pero cierra un hueco
  parecido): nadie puede crear/promover un ROOT nuevo desde ningún formulario -- ver
  commit `289dd40`.
- **Núcleo Rust -- login contra Supabase Auth** (`src/nube/auth_supabase.rs`, nuevo):
  - `login(base_url, apikey, cedula, password)` -- `POST /auth/v1/token?grant_type=password`
    con email sintético `<cedula>@brisas.local`.
  - `refrescar(base_url, apikey, refresh_token)` -- `grant_type=refresh_token`, para la
    renovación silenciosa en segundo plano.
  - `cambiar_password(...)` -- revalida la actual con un login real antes de aceptar la
    nueva (no confía en que la sesión siga abierta).
  - `obtener_jwks(base_url)` / `verificar_token_offline(claves, access_token)` --
    **separadas a propósito** (una hace red, la otra es pura) para poder testear la
    verificación de firma ES256 sin levantar nada. Confirmado con `curl` que el proyecto
    real (`xidaepyaljzkpbsxrqsm`) ya expone 3 claves EC en
    `/auth/v1/.well-known/jwks.json` -- no hace falta ningún cambio de configuración en
    Supabase para esto.
  - Dependencia nueva: `jsonwebtoken` (backend `ring`, sin OpenSSL -- no suma al costo de
    compilación ya documentado de SQLCipher).
  - **14 tests, todos pasando** (`cargo test --no-default-features --features
    terminal-ui,nube,sqlite-plano --lib auth_supabase`), incluida una firma real ES256
    contra un par de claves de prueba generado con `openssl ecparam` (ver el
    doc-comment en el archivo si hay que regenerarlo).
  - **Todavía NO está conectado a nada** -- es el módulo de bajo nivel solo. `services/`,
    `application/autenticacion.rs`, `SIN_PASSWORD_LOCAL`, `FijarPasswordInicial.tsx`, y el
    login de desktop siguen exactamente como antes.

**Lo que falta (el grueso del trabajo que queda):**

1. **Sesión en memoria** -- diseñar dónde vive el `access_token`/`refresh_token` mientras
   la app corre (en desktop, candidato natural: un campo nuevo en `GuiState`, junto a
   `sesion`/`token_nube_cacheado`). Nunca a disco. Se pierde al cerrar la app (arranque
   siempre pide login real).
2. **Renovación en segundo plano**: enganchar `refrescar()` al mismo pulso de sync que ya
   existe (`comandos/nube.rs::ejecutar_sincronizacion`, o un timer aparte) antes de que el
   token venza, y aplicar el tope duro de 12h de presencia (contado desde la última
   renovación exitosa, no desde el login).
3. **Reemplazar el camino de login actual** (`AutenticacionService::autenticar`/
   `buscar_candidato_autenticacion` en `src/services/autenticacion_service.rs`,
   `AppCore::fijar_password_inicial` en `src/application/autenticacion.rs`) -- OJO acá
   con la salvedad de ROOT: sólo el camino de usuarios SINCRONIZADOS (los que hoy llegan
   con `SIN_PASSWORD_LOCAL`) pasa a Supabase Auth. `crear_root_inicial` (arranque de un
   sitio nuevo, CLI/TUI, exige base vacía) queda intacto, 100% local, sin tocar -- es el
   único camino de rescate sin red. Un ROOT que SÍ llegó por sync a un dispositivo nuevo
   (con `SIN_PASSWORD_LOCAL`) sí pasa por el nuevo camino, igual que Administrador/Operador.
4. **Desktop**: `Login.tsx` pasa a ser una sola pantalla siempre (elimina
   `FijarPasswordInicial.tsx` como pantalla de "reclamo"); agregar el flujo de "cambiar
   contraseña obligatoria" cuando `debe_cambiar_password` viene en `true`; comandos Tauri
   nuevos para login/logout/cambiar-password que hablen con
   `nube::auth_supabase` en vez de con el núcleo local.
5. **Android**: fuera de alcance explícito por ahora (ver arriba), pero quedará
   desalineado del resto una vez desktop migre -- anotarlo cuando llegue el momento.

No hay decisión tomada todavía sobre el orden exacto de 1-4 -- son interdependientes
(la sesión en memoria la necesitan tanto el reemplazo del login como la renovación).
