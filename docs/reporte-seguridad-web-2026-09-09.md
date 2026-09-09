# Revisión de seguridad del panel y de la integración de visitas

Fecha: 2026-09-09. Rama de trabajo: `feature/control-visitas`.

## Alcance y conclusión

Revisión del código de `web/`, de las funciones administrativas relacionadas,
de políticas y permisos del proyecto Supabase real mediante consultas de solo
lectura, de dependencias y de la respuesta pública del panel. No se realizaron
escrituras de prueba, cambios de esquema, ataques de carga ni alteraciones del
panel publicado. Los hallazgos se entregan al responsable del backend.

El panel tiene defensas útiles: autenticación externa, autorización en base de
datos, verificación de identidad en las funciones administrativas y una política
CSP sin ejecución de scripts en línea. Sin embargo, algunos permisos del servidor
no coinciden con las restricciones que anuncia la interfaz. La integración nueva
de visitas tiene además un bloqueo de RLS confirmado.

## Hallazgos del panel

### P1 · Un administrador puede crear otros administradores por la API

**Confirmado en código y permisos de la base actual.** `web/src/App.tsx` explica
que se retiró la pantalla de administradores para que un compromiso del panel no
pueda agregar administradores. En la base siguen presentes las políticas
`admin_global agrega admins` (INSERT, `es_admin_global()`) y
`admin_global borra otros admins` (DELETE, otro correo y `es_admin_global()`).
`has_table_privilege('authenticated', 'public.administradores_panel', 'INSERT')`
y la misma consulta para DELETE devuelven `true`.

Una sesión de administrador robada o controlada conserva esas operaciones por
PostgREST aunque no haya botones que las ejecuten. Esto no significa que un
anfitrión o cualquier cuenta de Google sea administrador: la condición de admin
global sigue existiendo. El riesgo es persistencia y extensión de un compromiso
de una cuenta administrativa.

**Corrección propuesta:** si el alta/baja es realmente administrativa fuera del
panel, retirar los privilegios y políticas de escritura de los roles cliente,
conservar únicamente la lectura necesaria y probar la denegación con un token de
admin. Hacerlo como migración coordinada con el backend, incluyendo revisar el
webhook de sincronización de Access. Ocultar una pantalla no restringe una API;
la autorización de funciones debe aplicarse en el servidor.
[Referencia OWASP](https://owasp.org/API-Security/editions/2023/en/0xa5-broken-function-level-authorization/).

### P1 · La confirmación por correo de operaciones sensibles depende de la interfaz

**Confirmado por revisión del código local; falta comparar con la versión remota
de cada función.** `web/src/componentes/ConfirmacionSensible.tsx` llama primero a
`verifyOtp` y después a la mutación. `admin-revoke-device` y `admin-delete-device`
comprueban el token con `getUser` y la pertenencia a `administradores_panel`, pero
no comprueban que ese token incluya una reautenticación reciente ni una prueba
vinculada a la operación y al dispositivo.

El token administrativo previo a recibir el código resulta suficiente para
llamar directamente a esas funciones. El código de correo evita errores en el
flujo normal, pero no constituye una segunda barrera contra un token robado.

**Corrección propuesta:** definir y verificar la elevación de autenticación en el
servidor antes de la mutación: MFA con el nivel exigido o una prueba breve, de un
solo uso y ligada a actor, acción y destino. No confiar en un booleano del
navegador ni presentar un OTP de correo como equivalente automático a MFA.
Probar expresamente que el token anterior al desafío sea rechazado.

### P2 · Una consulta de autorización antigua puede restaurar estado de sesión

**Confirmado por análisis del código local.** `web/src/contexto/AuthContexto.tsx`
usa `vigente` para detectar el desmontaje, pero no diferencia las generaciones de
consultas. Si la consulta de la cuenta A termina después de SIGNED_OUT o del
cambio a B, puede ejecutar `setSesion` con A. Además, ante error de consulta se
conserva la sesión anterior sin comprobar que pertenezca a la misma identidad.

RLS sigue comprobando el token real y limita el alcance; este hallazgo por sí solo
no demuestra acceso a datos ajenos en el servidor. Sí puede restaurar una pantalla
o datos en memoria después del cierre/cambio de cuenta.

**Corrección propuesta:** invalidar consultas anteriores en cada cambio de sesión,
limpiar inmediatamente al cambiar identidad y conservar datos ante errores de red
solo cuando se trate de la misma cuenta. La web nueva incluye pruebas de estos
casos en `web-visitas/src/pruebas/auth.test.tsx`.

### P2 · Validación y errores de funciones administrativas necesitan endurecimiento

**Confirmado en el código local.** Por ejemplo, `admin-create-site/index.ts` tipa
`req.json()` con TypeScript pero no valida su forma en ejecución: `nombre: 123`
alcanza `.trim()` y puede producir una excepción; tampoco hay límites explícitos
de longitud en ese punto. Varias funciones devuelven `error.message` de Postgres
en `detail`, que el cliente muestra.

**Corrección propuesta:** esquema de entrada real, límite de tamaño del cuerpo,
UUID y longitudes, respuestas de error públicas estables y diagnóstico interno
sin documentos ni tokens. Evaluar límites de frecuencia por identidad y operación
en el servidor. No se enviaron cuerpos malformados a producción para reproducirlo.

## Bloqueos de integración de visitas

### P0 para publicar visitas · Recursión en RLS

**Reproducido en la base real en una transacción de solo lectura**, con rol
`authenticated` y correo ficticio. Consultar `anfitriones` produce:

```text
42P17: infinite recursion detected in policy for relation "citas"
```

Las políticas de `citas` y `cita_sitios` se consultan mutuamente; la política de
lectura de anfitriones por dispositivo también llega a ese ciclo. Esto bloquea
incluso comprobar si una persona está autorizada para entrar a la web.

La reproducción y los criterios de aislamiento están en
[contrato-web-visitas.md](contrato-web-visitas.md). Resolver desde el backend
sin desactivar RLS ni introducir acceso privilegiado en el navegador.
[Modelo de permisos de Supabase](https://supabase.com/docs/guides/database/postgres/row-level-security).

### P1 · Guardado, cancelación y baja de anfitriones requieren contrato del servidor

**Confirmado en el handoff y en el esquema revisado.** El flujo original propone
tres peticiones separadas para crear una cita. No son una transacción; puede
quedar una cita incompleta. La base todavía no tiene `crear_cita_anfitrion`.
La web implementa el cliente de esa RPC y conserva el mismo UUID en los reintentos;
su atomicidad e idempotencia solo estarán garantizadas al implementar el servidor.

La política UPDATE de `citas` controla el propietario, pero no restringe la
transición de estado: una llamada directa puede volver a escribir `VIGENTE`.
Tampoco se observan en la migración inicial límites de textos, cantidad de
visitantes o unicidad normalizada del documento. La validación del formulario
no sustituye esos controles.

Por último, `citas.anfitrion_correo` referencia `anfitriones.correo` sin borrado
en cascada. Eliminar físicamente un anfitrión con citas no es un mecanismo de
baja viable sin perder integridad. Definir una baja lógica o un mecanismo de
autorización separado que conserve el historial y deniegue tokens vigentes.

**Corrección propuesta:** implementar el contrato atómico, la transición de
cancelación y la revocación en servidor, con pruebas para dos anfitriones,
una cuenta sin alta y dispositivos de distintos sitios. Si se permite escritura
directa REST, las restricciones también deben cubrir ese camino; una validación
que solo vive dentro de la RPC puede eludirse.

## Cloudflare y resistencia a abuso

- La zona `megabrisas.com` está activa. Los dominios `megabrisas.com`,
  `www.megabrisas.com` y `panel.megabrisas.com` apuntan al Worker `panel-brisas`,
  según la consulta de dominios del proveedor.
- `https://panel.megabrisas.com` devuelve HTTP 302 hacia el dominio de Cloudflare
  Access. Eso confirma una barrera de entrada en esa URL, pero no el contenido
  de sus políticas ni la protección de otros orígenes.
- La consulta de reglas WAF devolvió 403 con la sesión disponible. La consulta
  de aplicaciones Access a nivel de cuenta devolvió una lista vacía a pesar de la
  redirección observada; no se interpreta como ausencia de protección. Queda por
  revisar la configuración de ámbito de zona y las políticas efectivas.
- No se verificaron reglas antiabuso, cobertura de dominios alternativos,
  desafíos, alertas, límites del plan ni comportamiento bajo carga. No se ofrece
  garantía de disponibilidad absoluta.

Para visitas se prepara un Worker de recursos estáticos, con `workers_dev` y
`preview_urls` deshabilitados y el dominio `visitas.megabrisas.com`. Al publicar,
configurar Access con su lista de anfitriones, comprobar que no herede una regla
de solo administradores y verificar que no haya un origen alternativo expuesto.
Configurar límites adecuados de peticiones y alertas según las reglas disponibles
en la cuenta. [Reglas de frecuencia de Cloudflare](https://developers.cloudflare.com/waf/rate-limiting-rules/).

La SPA llama directamente a Supabase. Las reglas de `megabrisas.com` no cubren
automáticamente `xidaepyaljzkpbsxrqsm.supabase.co`: la autorización, las cuotas y
la protección de escrituras deben existir también en ese servidor. Agregar un
proxy que deje abierta la API original no resolvería ese límite.

## Avisos del analizador de Supabase

Consulta de `get_advisors(type='security')`:

- `pg_net` instalado en `public`: revisar reubicación sin interrumpir los webhooks.
  [Detalle del aviso](https://supabase.com/docs/guides/database/database-linter?lint=0014_extension_in_public).
- `es_admin_global()` es `SECURITY DEFINER` y ejecutable por `authenticated`.
  La definición revisada devuelve solo pertenencia administrativa del llamador;
  no se detectó una escalada por sí sola. Revisar esquema privado y privilegios
  conservando las políticas que dependen de ella.
  [Detalle del aviso](https://supabase.com/docs/guides/database/database-linter?lint=0029_authenticated_security_definer_function_executable).
- Protección de contraseñas filtradas desactivada. Evaluar según los proveedores
  habilitados; el ingreso de esta web utiliza Google.
  [Configuración](https://supabase.com/docs/guides/auth/password-security#password-strength-and-leaked-password-protection).

## Controles implementados en la nueva web

Google OAuth con PKCE y retorno explícito al mismo origen, autorización contra
`anfitriones`, identidad comprobada con Auth, protección contra respuestas antiguas,
sesión en `sessionStorage` y bloqueo de escrituras mientras no se pueda verificar
el acceso. No almacena documentos ni formularios en almacenamiento persistente.
[Referencia del flujo PKCE](https://supabase.com/docs/guides/auth/sessions/pkce-flow).

CSP que permite solo scripts y estilos propios, sin `unsafe-inline` ni
`unsafe-eval`; bloqueo de iframes y objetos, `no-referrer`, `nosniff`, HSTS,
permisos de dispositivo deshabilitados y respuestas `no-store`. Los recursos se
compilan localmente y no cargan fuentes ni scripts de terceros. Las cabeceras
se prueban con el servidor estático real de Wrangler, además del navegador.
[Cabeceras de recursos estáticos](https://developers.cloudflare.com/workers/static-assets/headers/).

Fechas en Costa Rica, esquemas de entrada y respuesta, paginación de doce citas,
peticiones con plazo máximo, cancelación solo confirmada por la fila devuelta y
guardado con una única RPC. No hay reintentos de escritura automáticos ni
pantallas o interruptores de autenticación ficticia en producción. Las respuestas
simuladas se encuentran únicamente en las pruebas.

Zod se configura en modo `jitless` para evitar incluso las pruebas de compilación
dinámica que la CSP bloquearía. Las pruebas de navegador registran las infracciones
de CSP y exigen cero en todos los escenarios.

## Resultados de verificación de esta entrega

| Comprobación | Resultado |
| --- | --- |
| Pruebas de visitas (validación, fechas, API y sesión) | 27 aprobadas |
| Navegador Chromium de escritorio y móvil, servido por Wrangler | 12 aprobadas; cero infracciones CSP |
| Compilación de visitas | Aprobada |
| Pruebas existentes del panel | 70 aprobadas en 17 archivos |
| Compilación del panel | Aprobada; advertencia de tamaño de AG Grid, ya existente |
| Sincronización y contraste del diseño | Correctos; 56 combinaciones comprobadas |
| Auditoría npm del panel y de visitas | Cero vulnerabilidades conocidas reportadas |
| Simulación de despliegue de visitas | `wrangler deploy --dry-run` aprobado |
| Guardado real de citas / OAuth real de anfitriones | Pendientes del backend y configuración del subdominio |

La instalación inicial de Wrangler incluyó `sharp@0.35.2` a través de Miniflare,
con un aviso de seguridad en el procesador de imágenes usado por herramientas
locales. Se fijó `sharp@0.35.4`, se actualizó el lockfile y se repitieron la auditoría
y las pruebas del Worker con resultado satisfactorio. No se rebajó Wrangler ni
se desactivó la comprobación. [Aviso y versión corregida](https://github.com/advisories/GHSA-rgj7-g3m4-5g8c).

Estas verificaciones no sustituyen las pruebas del backend real con dos
anfitriones ni una prueba de disponibilidad bajo carga. La RPC requerida seguía
ausente en la última consulta de esta entrega.
