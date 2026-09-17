# Login de anfitriones (web-visitas): de "sólo Google" a cualquier dominio de correo

> Plan aprobado, sin ejecutar todavía — mismo criterio que otros documentos
> de esta carpeta (`plan-rediseno-web-visitas.md`,
> `plan-sesion-unica-dispositivos.md`).

## Contexto

`web-visitas` (portal de anfitriones para agendar visitas) hoy sólo permite
entrar con una cuenta de Google (`Continuar con Google`). El cliente tiene
colaboradores cuyo correo corporativo es de otro dominio (`.cof`, KOF) y no
quiere que dependan de tener Gmail para poder agendar sus visitas.

**Alcance de este plan, explícitamente acotado por el usuario:** sólo el
login de `web-visitas` (anfitriones). El login con cédula de los operadores
en la app de escritorio es un tema aparte, no se toca acá.

## Hallazgo clave: ya existe la pieza que resuelve esto

`web/src/componentes/useVerificacionPorCorreo.ts` ya implementa un login por
código de 6 dígitos al correo, usando el OTP nativo de Supabase
(`signInWithOtp` + `verifyOtp`, `type: "email"`) — funciona con **cualquier**
dominio de correo, no sólo Gmail, porque no depende de ningún proveedor
OAuth. Hoy se usa como "step-up" (confirmar acciones sensibles de un admin
ya logueado), pero el mecanismo de fondo es exactamente el que hace falta
para el login inicial de anfitriones.

Además, la autorización real de quién puede entrar **no depende de Google
en ningún lado**: `supabase/migrations/20260909181150_crea_control_de_visitas.sql`
define la política RLS de `anfitriones` como `using (auth.email() = correo)`
— agnóstica al proveedor. Y `AuthContexto.tsx` (línea ~84) ya hace el
verdadero gate de negocio consultando la tabla `anfitriones` por `correo`
después de que la sesión existe, sin importar cómo se creó esa sesión. Esto
significa que el cambio es quirúrgico: sólo hay que reemplazar **cómo se
crea la sesión**, no el resto del flujo de autorización.

## Cambios

### 1. `web-visitas/src/contexto/AuthContexto.tsx`

Reemplazar `iniciarSesion()` (hoy: `supabase.auth.signInWithOAuth({provider:
"google", ...})`) por dos funciones que envuelven el mismo patrón de
`useVerificacionPorCorreo`:

- `pedirCodigo(correo)` → `supabase.auth.signInWithOtp({ email: correo })`.
  **Diferencia importante con el step-up de admins**: ahí se usa
  `shouldCreateUser: false` porque el usuario ya existe y está logueado. Acá
  el anfitrión puede ser la primera vez que entra, así que hace falta
  `shouldCreateUser: true` (el default) para que Supabase le cree la cuenta
  al pedir el código. Esto no relaja la seguridad: el gate real siempre fue
  la fila en `anfitriones`, nunca el proveedor de auth — con Google también
  cualquiera con una cuenta de Google podía autenticarse, y quien no tenía
  fila en `anfitriones` quedaba igual afuera (`autorizar()` ya lo
  desloguea). Mismo trust boundary, sólo cambia el proveedor.
- `confirmarCodigo(correo, codigo)` → `supabase.auth.verifyOtp({ email,
  token: codigo, type: "email" })`.

Todo lo demás del archivo (`autorizar`, `programar`, `recuperar`,
`cerrarSesion`, el listener de `onAuthStateChange`) queda igual — opera
sobre la sesión ya creada, sin importar su origen.

Preferible extraer/reusar la lógica de `useVerificacionPorCorreo` en vez de
duplicar el manejo de errores (traducción al español, límite de reenvío)
que ya tiene resuelto.

### 2. `web-visitas/src/pantallas/Login.tsx`

Pasa de un botón único a un formulario de dos pasos (mismo espíritu que
`ConfirmacionSensible.tsx`, aunque no es reusable tal cual porque acá el
primer paso pide un correo, no sólo confirma una pregunta):

1. Campo de correo + botón "Enviar código".
2. Campo de código (6 dígitos, `inputMode="numeric"`,
   `autoComplete="one-time-code"`, igual que en `ConfirmacionSensible.tsx`)
   + botón "Confirmar".

Actualizar el copy ("Ingresá con la cuenta de Google que tenés autorizada"
→ algo neutral al dominio) y quitar la mención a Google.

### 3. Tests

- `web-visitas/src/pruebas/auth.test.tsx`: el test "maneja errores de OAuth"
  (línea 158) y los mocks de `signInWithOAuth` pasan a mockear
  `signInWithOtp`/`verifyOtp`. El resto de los tests (autorización server-
  side, deniega sin fila en `anfitriones`, error de red, carrera de
  sesiones) no dependen del proveedor y no deberían cambiar.
- `web-visitas/e2e/visitas.spec.ts`: revisar si simula el botón de Google
  para entrar; adaptar al nuevo flujo de dos pasos.

### 4. Nada que tocar en el backend/RLS

`anfitriones`, sus políticas RLS y las de `citas`/`cita_sitios`/
`cita_visitantes` ya son agnósticas al dominio de correo — no requieren
migración.

### 5. Verificar (fuera del código, antes de ejecutar)

`web-visitas/src/contexto/AuthContexto.tsx` ya hace
`window.location.assign("/cdn-cgi/access/logout")` para el hostname de
producción (`visitas.megabrisas.com`), lo que sugiere que ese dominio
también podría estar detrás de **Cloudflare Access** (Zero Trust) — no sólo
el panel admin, que sí confirmamos vía `supabase/functions/sync-access-policy/`
(pero esa función sólo sincroniza `administradores_panel`, nada de
`anfitriones`/visitas). Si Access también gatea `visitas.megabrisas.com`
con una política restringida a identidad de Google, ese gate está en el
dashboard de Cloudflare, fuera de este repo, y no se resolvería sólo
cambiando el código. **Confirmar con el usuario/dashboard de Cloudflare
antes de dar esto por completo** — si existe esa política, Access también
soporta "One-Time PIN" por correo como identity provider propio, así que
tiene solución, pero es un paso de configuración aparte.

## Verificación una vez implementado

- `npm test` en `web-visitas` (Vitest) — tests de `auth.test.tsx` en verde.
- Prueba manual: entrar con un correo `.cof` (no-Gmail) de punta a punta —
  pedir código, recibirlo, confirmarlo, ver que autoriza sólo si existe fila
  en `anfitriones`.
- Prueba manual del caso negativo: correo sin fila en `anfitriones` → debe
  seguir mostrando "Tu cuenta no está autorizada..." igual que hoy.
- `npx playwright test` (e2e) si se actualiza `visitas.spec.ts`.
