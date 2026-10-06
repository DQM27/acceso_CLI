# Agenda de visitas Brisas

Aplicación para anfitriones: ingresar con Google, avisar a la portería quién
viene, cuándo y a qué lugar, y ver cuándo llega cada persona. El registro físico
de entrada/salida pertenece a la aplicación del punto de acceso.

## Pantallas

| Ruta | Qué hace |
| --- | --- |
| `/visitas` | Mis visitas: las de hoy con el estado de cada persona («Llegó 9:12 · gafete 7», «Sin llegar», «Salió»), se refresca cada 30 s; y las próximas. |
| `/agendar` | Formulario de una sola página: ¿Quién viene? (con buscador de personas que ya vinieron), ¿Cuándo? (Hoy / Mañana / Varios días y hora estimada), ¿Dónde?, Motivo. Barra inferior con el resumen y el botón. `?desde=<id>` duplica una visita. |
| `/visitas/:id` | Detalle: personas y su estado, lugar, motivo; Editar, Duplicar y Cancelar. |
| `/visitas/:id/editar` | Editar. Sólo si la visita está vigente y nadie entró todavía. |
| `/historial` | Canceladas y pasadas, paginado. |

Las rutas viejas (`/citas`, `/nueva`) y la vuelta de Google (`/auth/callback`)
llevan a `/visitas`.

## Diseño

Misma familia visual que el panel: los tokens de `design/brisas.json` y los
controles compartidos, generados por `design/generar.mjs` en `src/diseno.css` y
`src/controles.css` (no editarlos a mano). Tailwind v4 para la maquetación; lo
propio vive en `@layer components` de `src/index.css` para que las utilidades
siempre ganen. Una columna de hasta 720 px, pensada primero para el teléfono.
Tema claro/oscuro con `data-theme` en `<html>` (clave `brisas:tema`), igual que
el panel. Textos en «usted» y horas en 24 h.

## Escrituras: sólo por funciones de la base

El anfitrión no escribe tablas directamente: la migración
`20261005130000_visitas_web_anfitrion.sql` quitó esas políticas. Todo pasa por
funciones que validan dueño, fechas, límites y cuenta activa:

- `crear_cita_anfitrion`: idempotente por el id que genera el cliente; un
  reintento con el mismo contenido devuelve la misma cita.
- `editar_cita_anfitrion`: cancela la vieja y crea la nueva en un solo paso, para
  que el cambio llegue a todas las porterías (la sincronización del núcleo no
  borra visitantes). Se rechaza si alguien de la visita ya entró.
- `cancelar_cita_anfitrion`.
- `estado_visitantes_de_mis_citas`: quién llegó, sin dar acceso a los
  movimientos de la portería.
- `visitantes_anteriores`: personas que ese anfitrión ya agendó.

**Orden de despliegue:** esa migración y esta web salen juntas. La web anterior
cancelaba con un `UPDATE` directo que la migración deshabilita.

## Organización

- `web/` conserva el panel administrativo y su despliegue actual.
- `web-visitas/` es esta aplicación, con dependencias y despliegue independientes.
- `src/diseno.css` y `src/controles.css` los genera `design/generar.mjs`;
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
(escritorio y móvil), autorización denegada, Mis visitas con llegadas,
historial, agendar con validación, duplicados, dos lugares y reintento
idempotente, personas que ya vinieron, editar, cancelar, salir sin guardar y
sin conexión. El reloj se fija en una fecha, y cualquier violación de la CSP
hace fallar la prueba. Las capturas de cada pantalla quedan en `test-results/`
(ignorado por Git).

Los avisos breves («Visita agendada») son propios (`src/avisos.ts`) y no una
librería: la CSP (`style-src 'self'`) no deja inyectar estilos.

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

«Hoy» y «Próximas» son las citas `VIGENTE` cuya fecha final es hoy o después
(día de Costa Rica). El historial (canceladas o con fecha final pasada) se pagina
en el servidor. «Pasada» se calcula, nunca se guarda como un tercer estado.

El formulario admite hasta 50 visitantes y 100 sitios por cita; no determina un
máximo de días de negocio. Los límites, la normalización del documento y el
guardado atómico deben coincidir con el servidor. Los controles del cliente no
constituyen por sí solos una barrera frente a llamadas directas a la API.

## Estado de integración y publicación

Dominio: **visitas.megabrisas.com**. Worker: **visitas-brisas**.

El frontend está implementado. El backend (recursión RLS y la RPC atómica
`crear_cita_anfitrion` descritas en [el contrato de backend](../docs/auditorias/contrato-web-visitas.md))
ya está resuelto y probado -- ver
[la respuesta del backend](../docs/auditorias/respuesta-contrato-web-visitas.md), 2026-09-09.
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
entrega. [Reporte de seguridad](../docs/auditorias/reporte-seguridad-web-2026-09-09.md).
