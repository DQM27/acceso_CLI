# Agenda de visitas Brisas

Aplicación para anfitriones: ingresar con Google, agendar personas y grupos en
uno o varios sitios, consultar citas propias y cancelar. El registro físico de
entrada/salida pertenece a la aplicación del punto de acceso.

## Organización

- `web/` conserva el panel administrativo y su despliegue actual.
- `web-visitas/` es esta aplicación, con dependencias y despliegue independientes.
- `design/brisas.css` se importa directamente desde el diseño compartido generado;
  `design/brisas.json` y `design/controles.css` siguen siendo la fuente de verdad.
- No importa código del núcleo, escritorio, móvil ni del panel.

## Desarrollo y pruebas

Node 24 o posterior:

```powershell
cd web-visitas
npm ci --ignore-scripts
npm run dev
```

Abre `http://127.0.0.1:5174`. Para OAuth local, autorizar el retorno exacto
`http://127.0.0.1:5174/auth/callback` en un entorno de Supabase apropiado.

```powershell
npm test
npm run build
npx playwright install chromium
npm run test:e2e
npx wrangler deploy --dry-run
```

Las pruebas de navegador ejecutan el paquete de producción con Wrangler local
en `127.0.0.1:8791`, con las cabeceras de seguridad reales. Interceptan Supabase
con personas ficticias; no escriben en el proyecto real. Cubren temas, tamaños
de pantalla, autorización denegada, formulario, duplicados, reintentos y
cancelación. Las capturas quedan en `test-results/` (ignorado por Git).

El flujo de GitHub Actions `.github/workflows/web.yml` verifica ambas aplicaciones
en cada cambio relevante. No publica automáticamente.

Wrangler 4.129.0 trae `sharp@0.35.2` mediante Miniflare. El `override` acotado en
`package.json` exige la corrección compatible `sharp@0.35.4` para
[GHSA-rgj7-g3m4-5g8c](https://github.com/advisories/GHSA-rgj7-g3m4-5g8c).
Se utiliza solo en desarrollo; no forma parte del paquete del navegador.
Revisar si se puede retirar al actualizar Miniflare a una versión que ya incorpore
esa corrección. Se valida con las pruebas completas de Wrangler y `npm audit`.

## Configuración y seguridad

`src/lib/supabase.ts` contiene exclusivamente la URL y la clave **publishable**
indicadas en el handoff. Nunca agregar claves administrativas al cliente, al
directorio `public/` ni a variables `VITE_*`.

La sesión se guarda solo en la pestaña y se consulta el acceso al iniciar, al
cambiar el token, al volver a la pestaña y cada minuto visible. Un problema de
conexión conserva el formulario de la misma cuenta y bloquea las escrituras.
Cambiar de identidad o cerrar sesión limpia la interfaz. La preferencia de tema
es el único dato que se guarda en `localStorage`.

La lista se pagina en el servidor. El filtro «Vigentes» incluye citas futuras
cuyo estado es `VIGENTE`; «Vencidas» compara la fecha final con el día actual en
Costa Rica y nunca persiste un tercer estado.

El formulario admite hasta 50 visitantes y 100 sitios por cita; no determina un
máximo de días de negocio. Los límites, la normalización del documento y el
guardado atómico deben coincidir con el servidor. Los controles del cliente no
constituyen por sí solos una barrera frente a llamadas directas a la API.

## Estado de integración y publicación

Dominio: **visitas.megabrisas.com**. Worker: **visitas-brisas**.

El frontend está implementado. El backend (recursión RLS y la RPC atómica
`crear_cita_anfitrion` descritas en [el contrato de backend](../docs/contrato-web-visitas.md))
ya está resuelto y probado -- ver
[la respuesta del backend](../docs/respuesta-contrato-web-visitas.md), 2026-09-09.
El código sigue fallando de forma cerrada cuando el servicio no permite comprobar
la autorización, y no sustituye el guardado atómico por varias inserciones.

Antes de publicar:

1. ~~El responsable del backend implementa y prueba el contrato~~ -- hecho, ver
   la respuesta del backend enlazada arriba. Repetir igual las pruebas de dos
   anfitriones / cuenta sin alta / dispositivo de otro sitio contra el entorno
   real antes de anunciar el dominio.
2. Registrar `https://visitas.megabrisas.com/auth/callback` entre las URLs de retorno
   de Supabase. Mantener la URL principal del panel y evitar comodines amplios.
3. Configurar Cloudflare Access y las reglas antiabuso para este subdominio;
   comprobar usuarios permitidos, cierre de Access, WAF y dominios alternativos.
   La protección de Supabase se configura separadamente del dominio de la web.
4. Volver a ejecutar las verificaciones y desplegar con `npx wrangler deploy`.
5. Comprobar HTTPS y cabeceras en la URL final; completar un login real con Google
   y la prueba funcional con dos anfitriones de prueba autorizados.

No se cambió el esquema ni se publicaron recursos en producción durante esta
entrega. [Reporte de seguridad](../docs/reporte-seguridad-web-2026-09-09.md).
