# Investigación: reemplazar Google por correo y contraseña en la web de visitas

Fecha: 2026-10-06 · Alcance: `web-visitas/` (visitas.megabrisas.com) y el
proyecto Supabase compartido `xidaepyaljzkpbsxrqsm`.

**Pedido del cliente:** quitar «Continuar con Google» y usar un ingreso
tradicional con el correo de dominio propio que ya tienen los anfitriones.

**Estado:** solo investigación y documentación. No se cambió código,
esquema ni configuración de producción. Las consultas a producción fueron
de solo lectura y agregadas (conteos por dominio, sin datos personales).

---

## 1. Resumen ejecutivo

El cambio es viable. El riesgo real **no está en el formulario**. Está en que
toda la autorización del proyecto decide por el correo del JWT
(`auth.email()`): anfitriones, administradores del panel y sus políticas
RLS. Con Google, el correo lo verifica Google. Con contraseña, el correo lo
declara quien se registra, y Supabase solo lo da por bueno si la
configuración de Auth lo obliga a confirmarlo.

Si el proyecto permite **registro público** (`/auth/v1/signup` con la
publishable key, que es pública) **sin confirmación de correo**, cualquiera
puede crear una cuenta con el correo de un anfitrión o de un administrador
que todavía no tiene usuario en Auth, y entrar con esa identidad. Esto es
una **toma de cuenta previa** (*pre-account takeover*). En el panel equivale
a obtener privilegios de `admin_global`.

**El riesgo no es solo teórico hoy:** el proveedor de correo y contraseña
**ya está activo** en producción, porque los operadores entran con
`<cedula>@brisas.local`. Un administrador del panel figura en
`administradores_panel` y todavía no tiene usuario en Auth. La auditoría del
2026-09-24 ya marcó este punto como **NS-05** y quedó pendiente de verificar
en el dashboard.

**Recomendación:** hacer el cambio solo junto con estos controles
obligatorios:

1. Desactivar el registro público («Allow new users to sign up»).
2. Crear las cuentas solo por invitación de administración.
3. Activar la confirmación de correo y el cambio de correo seguro.
4. Configurar un SMTP propio.
5. Agregar un *hook* «Before User Created» que rechace correos que no estén
   autorizados.
6. Usar enlaces de correo con `token_hash` que la persona canjea con un clic.

El detalle está en la §5.

---

## 2. Cómo funciona hoy (verificado en el código)

| Pieza | Situación actual |
| --- | --- |
| Ingreso | `signInWithOAuth({ provider: "google" })` en `web-visitas/src/contexto/AuthContexto.tsx`; retorno a `/auth/callback` (PKCE). |
| Sesión | Solo en `sessionStorage` de la pestaña (`src/lib/supabase.ts`). |
| Autorización | Después de iniciar sesión, `getUser()` y búsqueda en `public.anfitriones` por correo. En la base, `private.anfitrion_activo_que_llama()` usa `auth.email()` y exige `activo`. |
| Panel | También autoriza por `auth.email()` contra `administradores_panel` (varias migraciones desde `20260905024834`). |
| Operadores | Ya usan correo y contraseña: `admin-create-usuario` crea `<cedula>@brisas.local` con la API de administración y `email_confirm: true`. |
| Cloudflare Access | El README de `web-visitas` pide poner Access delante del subdominio. Si su proveedor de identidad es Google, sigue exigiendo Google aunque la web ya no lo use. |
| `supabase/config.toml` (local) | `enable_signup = true`, `enable_confirmations = false`, `minimum_password_length = 6`. **No es la configuración de producción**, pero si alguien la tomó como referencia, el riesgo de la §1 existe. |

### Datos de producción (solo lectura, 2026-10-06)

- `auth.users`: 5 cuentas `email` en `@brisas.local` (operadores) y 3
  cuentas `google` en `@gmail.com`. **No se encontraron cuentas creadas por
  registro público** ni cuentas sin confirmar.
- `anfitriones`: 1 fila (`@gmail.com`). `administradores_panel`: 3 filas
  (`@gmail.com`); **una todavía no tiene usuario en Auth**.
- Todavía no hay anfitriones con el dominio propio del cliente. Hay que
  confirmar ese dominio con él.
- Advisor de seguridad: **«Leaked Password Protection Disabled»** sigue
  abierto.

---

## 3. Amenazas que abre el cambio

| # | Amenaza | Impacto | Mitigación (§5) |
| --- | --- | --- | --- |
| A1 | Registro público con el correo ajeno de un anfitrión o administrador que aún no existe en Auth | Suplantación; con un correo de administrador, control del panel | C1, C2, C3, C5 |
| A2 | Registro público de `<cedula>@brisas.local` (NS-05) | Entrar como ROOT u operador en equipos con `SIN_PASSWORD_LOCAL` | C1 (lo cierra; el *hook* no distingue el alta por API) |
| A3 | Fuerza bruta o *credential stuffing* (contraseñas reutilizadas de otras filtraciones) | Toma de cuenta | C6, C7, C8, C9 |
| A4 | Enumeración de cuentas (mensajes distintos para «no existe» y «clave mala», o en «olvidé mi contraseña») | Lista de anfitriones para *phishing* | C10 |
| A5 | Enlaces de recuperación o invitación consumidos por escáneres de correo corporativo (Safe Links, Mimecast, Proofpoint) | El enlace llega gastado; soporte constante | C11 |
| A6 | Enlace con PKCE abierto en otro navegador o pestaña (la sesión vive en `sessionStorage`) | El enlace falla siempre | C11 |
| A7 | Token del enlace expuesto en registros, historial o `Referer` | Reutilización dentro de la vigencia | C11 (fragmento `#`, borrar la URL, `Referrer-Policy: no-referrer` ya existe) |
| A8 | Cambio de correo de la cuenta hacia el correo de otra persona | Suplantación | C4 |
| A9 | SMTP por defecto de Supabase: 2 correos por hora y solo a miembros del equipo | Nadie recibe invitaciones ni recuperaciones | C12 |
| A10 | Contraseña única sin segundo factor | Una sola filtración basta | C7, C13 (MFA opcional, fase 2) |
| A11 | Cloudflare Access configurado con Google | El usuario no puede pasar del primer paso | C14 |

---

## 4. Qué dicen las fuentes

- **NIST SP 800-63B-4 (final, agosto de 2025):**
  - Si la contraseña es el único factor, el mínimo es 15 caracteres (8 con
    MFA).
  - Se deben admitir al menos 64 caracteres.
  - No se deben imponer reglas de composición (mayúscula, número, símbolo).
  - No se debe exigir caducidad periódica.
  - Hay que comparar contra una lista de contraseñas comunes o filtradas.
  - Se debe permitir pegar y mostrar la contraseña.
- **OWASP (Authentication Cheat Sheet):**
  - Mensaje genérico en el ingreso («correo o contraseña incorrectos»).
  - Misma respuesta en «olvidé mi contraseña» exista o no la cuenta.
  - Tokens de un solo uso y de vida corta.
  - Límite de intentos por IP y por cuenta, y CAPTCHA ante abuso.
- **Supabase:**
  - `resetPasswordForEmail` no revela si la cuenta existe.
  - Con PKCE, la plantilla de correo debe llevar `{{ .TokenHash }}` hacia una
    ruta que llame a `verifyOtp({ token_hash, type })`, para que funcione en
    cualquier navegador.
  - Los escáneres consumen los enlaces. La solución recomendada es una
    página intermedia que solo canjea tras un clic, con el token en el
    fragmento.
  - El SMTP por defecto no sirve en producción: un SMTP propio es
    obligatorio.
  - La protección contra contraseñas filtradas (HaveIBeenPwned) requiere el
    plan Pro.
  - El *hook* «Before User Created» permite rechazar altas (devuelve
    `{"error":{"http_code":403,...}}`), se ejecuta como `supabase_auth_admin`
    y se activa en el dashboard.
  - CAPTCHA nativo con Turnstile o hCaptcha para ingreso, registro y
    recuperación.

---

## 5. Controles recomendados

**Obligatorios antes de publicar (C1–C12, C14).** Ningún control reemplaza a
otro.

| # | Control | Dónde | Notas |
| --- | --- | --- | --- |
| C1 | **Desactivar «Allow new users to sign up»** | Dashboard → Authentication → Sign In / Providers | Cierra A1 y A2. La API de administración (operadores, invitaciones) sigue funcionando. **Efecto en el panel:** un administrador nuevo de Google ya no podrá crear su cuenta en el primer ingreso. Hay que invitarlo antes. Probarlo en staging. |
| C2 | **Alta solo por invitación** | Dashboard → Users → «Invite user», o a futuro una Edge Function con `inviteUserByEmail` que verifique `admin_global` | Orden: primero la fila en `anfitriones`, después la invitación. |
| C3 | **Confirmación de correo activa** | Dashboard → Email → «Confirm email» | Ninguna cuenta obtiene sesión sin probar que el buzón es suyo. |
| C4 | **«Secure email change» activo** | Dashboard → Email | El cambio de correo se confirma en ambos buzones. |
| C5 | **Hook «Before User Created»** que solo permita correos en `anfitriones` (activos), en `administradores_panel` o en `@brisas.local` | Migración (función `security definer`, `execute` solo para `supabase_auth_admin`) y activación en el dashboard | Defensa en profundidad si alguien reactiva el registro. Comparar en minúsculas. |
| C6 | **Límites de intentos** | Dashboard → Rate Limits | Revisar «sign-ins/sign-ups» y «token verifications». Para el ingreso, Supabase limita por IP. |
| C7 | **Contraseñas: mínimo 15, sin reglas de composición, rechazar filtradas** | Dashboard → Passwords; además, validación en la web | **Ojo:** el mínimo del servidor rige para todos. Las contraseñas temporales de operadores son de 10 caracteres (`admin-create-usuario`). Subir el mínimo exige primero alargar ese generador. La comprobación de filtradas requiere el plan Pro. |
| C8 | **CAPTCHA (Cloudflare Turnstile)** en ingreso y recuperación | Dashboard → Attack Protection; `captchaToken` en el cliente | Requiere abrir `challenges.cloudflare.com` en la CSP (`script-src` y `frame-src`). Se puede posponer si C6 y Cloudflare WAF cubren el riesgo. |
| C9 | **Reglas WAF y de límite de Cloudflare** sobre el subdominio | Cloudflare | Ya figura en el README. Las llamadas de Auth van directo a Supabase, así que esto protege la web, no `/auth/v1`. |
| C10 | **Mensajes genéricos** | Web | «Correo o contraseña incorrectos» para cualquier 4xx. Recuperación: «si el correo corresponde a una cuenta, recibirá un enlace». |
| C11 | **Plantillas de correo con `token_hash` en el fragmento y canje con un clic** | Dashboard → Email Templates («Invite user» y «Reset password»); ruta `/auth/confirmar` en la web | Ejemplo: `https://visitas.megabrisas.com/auth/confirmar#token_hash={{ .TokenHash }}&type=recovery`. La página borra el fragmento de la URL y llama a `verifyOtp` solo al pulsar «Continuar». Con esto, `detectSessionInUrl` puede pasar a `false`. |
| C12 | **SMTP propio** (del dominio del cliente o de un proveedor como Resend o SES) con SPF, DKIM y DMARC | Dashboard → SMTP | Sin esto no llegan invitaciones ni recuperaciones. |
| C13 | MFA TOTP opcional para anfitriones | Fase 2 | NIST baja el mínimo a 8 caracteres si hay MFA, pero se recomienda mantener 15. |
| C14 | **Cloudflare Access:** usar «One-time PIN» con la lista de correos o el dominio autorizado, o quitar Access de este subdominio | Cloudflare Zero Trust | Si queda con Google, el cambio no sirve. |

### Detalles de la web (si se aprueba implementar)

- Formulario con `type="email"` y `autocomplete="username"`; contraseña con
  `autocomplete="current-password"` / `"new-password"`.
- Botón para mostrar la contraseña; nunca bloquear el pegado.
- Normalizar el correo (`trim().toLowerCase()`) antes de enviarlo.
- Nunca guardar la contraseña en el estado global ni en almacenamiento. La
  sesión se mantiene en `sessionStorage`, como hoy.
- Pantalla «crear o recuperar la contraseña»: sirve también para que los
  anfitriones que hoy entran con Google definan su contraseña (Supabase
  permite agregar una contraseña a una cuenta OAuth existente).
- Eliminar la ruta `/auth/callback` de las URL de retorno de Supabase cuando
  ya nadie use Google en esta web.
- Actualizar las pruebas unitarias y e2e (hoy buscan «Continuar con Google»).

---

## 6. Verificaciones pendientes en el dashboard (no accesibles por MCP)

La configuración de Auth no se puede leer con las herramientas usadas. Hay
que comprobarla a mano en producción, **independientemente de este
cambio**, porque A2 (NS-05) ya aplica hoy:

- [ ] «Allow new users to sign up»: ¿activado?
- [ ] «Confirm email»: ¿activado?
- [ ] «Secure email change»: ¿activado?
- [ ] SMTP propio: ¿configurado?
- [ ] Mínimo de contraseña y requisitos actuales.
- [ ] Proveedor de identidad de Cloudflare Access en `visitas.megabrisas.com`.

Si las dos primeras están en «registro activado / confirmación desactivada»,
corregirlo es **urgente** aunque no se haga el cambio de login.

---

## 7. Decisiones que debe tomar el cliente o el responsable

1. Dominio exacto del correo de los anfitriones (para el *hook* y los
   textos).
2. ¿Plan Pro de Supabase? Lo requiere la protección contra contraseñas
   filtradas; también da más control sobre la duración de las sesiones.
3. Proveedor SMTP y remitente (por ejemplo, `no-responder@<dominio>`).
4. ¿El panel administrativo sigue con Google? Si es así, aceptar que los
   administradores nuevos requieran invitación previa (efecto de C1).
5. ¿CAPTCHA ahora o en una segunda fase?

---

## Fuentes

- [NIST SP 800-63B-4: requisitos de contraseñas (análisis de la versión final)](https://www.sakimura.org/en/2025/10/7710/)
- [NIST Scraps Passwords Complexity and Mandatory Changes – Infosecurity Magazine](https://www.infosecurity-magazine.com/news/nist-scraps-passwords-mandatory/)
- [Supabase – Password-based Auth](https://supabase.com/docs/guides/auth/passwords)
- [Supabase – Before User Created Hook](https://supabase.com/docs/guides/auth/auth-hooks/before-user-created-hook)
- [Supabase – Custom SMTP](https://supabase.com/docs/guides/auth/auth-smtp)
- [Supabase – CAPTCHA protection](https://supabase.com/docs/guides/auth/auth-captcha)
- [Supabase – Password security / leaked password protection](https://supabase.com/docs/guides/auth/password-security#password-strength-and-leaked-password-protection)
- [Supabase – OTP verification failures (email prefetching)](https://supabase.com/docs/guides/troubleshooting/otp-verification-failures-token-has-expired-or-otp_expired-errors-5ee4d0)
- [Supabase discussion #41618 – enlaces consumidos por escáneres de correo](https://github.com/orgs/supabase/discussions/41618)
- [Supabase – General configuration (deshabilitar registros)](https://supabase.com/docs/guides/auth/general-configuration)
- [MakerKit – Plantillas con token_hash para PKCE](https://makerkit.dev/docs/next-supabase-turbo/emails/authentication-emails)
- [OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)
- Interno: `docs/auditorias/auditoria-integral-2026-09-24/02-nube-supabase.md` (NS-05), `docs/arquitectura/arquitectura-supabase.md` §6.4.
