# Plan: veto de acceso por persona (cédula), en todas las puertas

Estado: **propuesto, sin implementar** (2026-09-27). Reemplaza como plan de
trabajo al pendiente "No existe un veto de acceso por cédula que cruce
catálogos" de `docs/pendientes.md`; aquel diseño se tomó como referencia,
no como decisión.

## 1. El problema

Hoy la única forma de negarle el acceso a alguien es
`contratistas.tiene_acceso` (Regla 1 de `domain::acceso::verificar_acceso`).
El veto está atado al **rol** y no a la **persona**:

| Hueco | Dónde | Efecto |
|---|---|---|
| Proveedores no consultan ningún veto | `IngresoProveedorService::registrar_ingreso` ("sin catálogo de personas, cédula/nombre son snapshot puro") | Un contratista vetado entra como proveedor con la misma cédula |
| Visitas no consultan ningún veto | `CitaService::verificar_check_in` | Igual, entra como visita |
| La cédula no tiene forma canónica | Contratistas: sólo dígitos (`domain::contratista::cedula_valida`), pero `0112340567` ≠ `112340567`. Proveedores y visitas: sólo `trim()` | Aunque existiera el veto, `1-1234-0567` lo esquiva |
| "Un ingreso activo" no cruza de rol | `contratista_con_ingreso_activo` mira sólo `ingresos`; proveedores sólo miran `registro_ingresos_proveedor` | Un contratista adentro puede entrar además como proveedor |
| `tiene_acceso` mezcla conceptos | "esta relación laboral está autorizada" vs. "esta persona no entra" | Si el contratista deja de serlo y vuelve como proveedor, el veto no lo sigue |

Mismo hallazgo en la auditoría integral (NR-10, CWE-1289: validación
insuficiente de entradas equivalentes).

## 2. Qué hace la industria

Los sistemas de gestión de visitas y contratistas (HID Visitor Manager,
FacilityOS/VisitorOS, Visitly, RightCrowd) coinciden en:

- **Una lista interna de personas restringidas** ("watchlist", "deny
  list"), separada de los catálogos por rol, que se consulta en **todo**
  registro, sea visita, contratista o proveedor.
- **Identidad única por persona** para todos los roles. El cruce se hace
  por el número de documento, no por el nombre (el nombre da falsos
  positivos y se esquiva con apodos).
- **Dos niveles de respuesta:** negar automáticamente, o alertar y exigir
  que seguridad apruebe. Aquí se usa sólo el primero: niega siempre, y
  sólo el administrador lo levanta.
- **Registro de quién puso o quitó cada restricción, cuándo y por qué.**

Diferencia a favor de este proyecto: los sistemas comerciales dependen de
estar en línea. Aquí el veto tiene que funcionar **sin internet**, así
que viaja al equipo como el resto del catálogo.

## 3. Decisiones

| Decisión | Valor | Origen |
|---|---|---|
| Quién veta y levanta | **Sólo el administrador del panel web** (`es_admin_global()`) | Dueño, 2026-09-27 |
| Alcance | Global: todos los sitios (igual que contratistas y empresas) | Propuesta |
| Puertas | Contratista, proveedor, visita y gafete provisional KOF | Propuesta |
| Respuesta | Negar siempre. La portería no puede pasar por encima | Propuesta |
| Qué ve el operador | Mensaje genérico ("acceso restringido, comuníquese con administración"), sin el motivo | Propuesta (privacidad, ver §8) |
| `tiene_acceso` | Se conserva con su significado: autorización laboral del contratista | Propuesta |

## 4. Identidad: la cédula canónica

Sin esto, el veto no sirve. Es la **fase 1**.

Tipo `Cedula` en el núcleo (`src/domain/cedula.rs`), con un único
constructor que normaliza:

1. Quitar espacios, guiones y puntos. Pasar a mayúsculas.
2. Formato del TSE con cero inicial: 10 dígitos que empiezan en `0` →
   quitar ese cero (`0112340567` → `112340567`).
3. Clasificar sin rechazar: 9 dígitos = cédula física; 11 o 12 dígitos =
   DIMEX; alfanumérico de 5 a 20 caracteres = pasaporte u otro documento.
   Cualquier otra cosa es un error.

Se usa en **todos** los puntos de entrada: alta de contratista, ingreso de
proveedor, check-in de visita, veto, y la sincronización (lo que baja de
la nube también se normaliza).

Supabase tiene una función SQL gemela `public.normalizar_cedula(text)`,
para las RPC y los índices. Las dos se prueban con el **mismo** juego de
casos (un archivo de vectores compartido), para que nunca discrepen.

**Datos existentes:** antes de normalizar, un diagnóstico (consulta de
sólo lectura, local y en staging) lista las cédulas que chocarían, por
ejemplo dos contratistas que en realidad son la misma persona. El dueño
decide cómo fusionarlas. No se fusiona nada automáticamente.

## 5. Datos

### Supabase (primero staging, nunca producción sin autorización)

```
personas_vetadas
  id              uuid pk
  cedula          text not null          -- ya canónica
  nombre_ref      text                   -- sólo referencia visual
  motivo          text not null          -- visible sólo para administradores
  vetado_por      uuid not null          -- administrador del panel
  vetado_en       timestamptz not null
  levantado_por   uuid
  levantado_en    timestamptz
  motivo_levanta  text
  updated_at      timestamptz not null
  unique (cedula) where levantado_en is null   -- un veto vigente por persona
```

- Levantar un veto no borra la fila; queda la historia completa. Volver a
  vetar crea una fila nueva.
- **RLS:** los dispositivos sólo pueden leer `id, cedula, vigente,
  updated_at`, sin motivo. Escribir sólo es posible por las RPC
  `vetar_persona(cedula, nombre, motivo)` y `levantar_veto(cedula,
  motivo)`, `security definer`, que exigen `es_admin_global()` y
  normalizan la cédula.
- Aviso en vivo: el trigger de `cambio_nube` cubre también esta tabla
  (con la fila, igual que `ingresos` en `realtime-con-datos`). Un veto
  llega a las porterías en segundos.

### SQLite (equipos)

- Migración nueva (`personas_vetadas`: `uuid`, `cedula`, `vigente`,
  `actualizado_en`) con índice por `cedula`.
- Baja con el catálogo, página por página (patrón de N6), con marca de
  agua propia (`vetos_actualizado_hasta`). Baja también los vetos
  levantados, para poder quitarlos localmente.
- Móvil y escritorio lo guardan igual. Es liviano: sólo cédulas.

## 6. La regla (en el núcleo, como todas)

- `domain::acceso`: nueva **Regla 0** que va antes que todas:
  `persona vetada → Denegado(PersonaVetada)`. Es una función pura que
  recibe "¿está vetada?" y no consulta la base.
- Se evalúa en **todas** las puertas: `preparar_ingreso` y
  `registrar_entrada` (contratista), `registrar_ingreso` (proveedor),
  `entregar` (gafete provisional KOF) y `verificar_check_in` (visita).
  Todo pasa por el núcleo, así que escritorio y móvil sólo muestran el
  resultado.
- **Con internet:** la verificación en vivo que ya existe
  (`preparar_ingreso_verificado` y compañía) agrega el veto a la misma
  consulta a la nube. No suma viajes de red.
- **Sin internet:** decide la copia local. Un veto recién puesto sin
  internet en la portería llega en la próxima conexión.
- Mensaje en `src/mensajes.rs`: uno solo para todos los roles, sin
  motivo.
- Auditoría local: cada intento bloqueado queda registrado (cédula,
  puerta, operador, hora) y sube a la nube. El administrador ve quién
  intentó entrar estando vetado.

## 7. Un ingreso activo por persona, entre roles

Se extiende la regla que ya existe:

- Local: antes de registrar, se busca la cédula canónica en
  `registro_ingresos`, `registro_ingresos_proveedor`,
  `movimientos_visita` y sus cachés remotas.
- Nube: `contratista_con_ingreso_activo` pasa a llamarse
  `persona_con_ingreso_activo` y mira las tres tablas.
- Mensaje: "ya está adentro como proveedor en <sitio>".

## 8. Panel web: pantalla "Personas restringidas"

- Buscar por cédula (se normaliza al escribir). Muestra si la persona
  aparece como contratista, proveedor o visita, y si está adentro ahora.
- Vetar: motivo obligatorio y confirmación.
- Levantar: motivo obligatorio.
- Lista de vigentes y de historial, con quién y cuándo.
- Intentos bloqueados recientes.

**Privacidad (Ley 8968 de Protección de Datos de Costa Rica):** el motivo
es un dato sensible. Sólo lo ve el administrador. A los equipos no viaja;
la portería sólo sabe que la persona no puede entrar. Motivos cortos y
objetivos.

## 9. Fases

Cada fase va en su propia rama derivada de N1, con pruebas, y sólo se une
después de que el dueño la pruebe.

| Fase | Contenido | Toca |
|---|---|---|
| F1 | Cédula canónica + diagnóstico de duplicados | Núcleo, sincronización |
| F2 | Datos del veto: tabla, RLS, RPC y aviso en vivo (staging); migración local y descarga | Supabase staging, núcleo |
| F3 | Regla 0 en las cuatro puertas + mensajes + auditoría de intentos | Núcleo, escritorio, móvil (sólo muestran) |
| F4 | Pantalla del panel web | Panel web |
| F5 | Un ingreso activo por persona entre roles | Núcleo, nube |

F1 sirve por sí sola: cierra que la misma persona aparezca dos veces con
otro formato. F2 y F3 juntas cierran el hueco principal.

## 10. Riesgos

- **Duplicados al normalizar:** se detectan antes (F1) y se fusionan con
  el dueño. Nunca en silencio.
- **Veto puesto sin internet en la portería:** llega en la próxima
  conexión. Es el mismo límite que ya tiene todo el catálogo.
- **Bindings de UniFFI:** cada cambio del puente móvil exige regenerarlos
  (el CI lo verifica).
- **Pasaportes:** sin dígito verificador, dependen de que el operador
  copie bien el documento. El OCR ayuda.

## Referencias

- HID Visitor Manager, Watchlist: https://docs.hidglobal.com/hid-visitor-manager/content/guides/watchlist.htm
- FacilityOS, Watchlist management: https://www.facilityos.com/visitoros/watchlist-management
- Visitly, watchlist para visitas y contratistas: https://www.visitly.io/blogs/automated-watchlist-screening-visitors-contractors-visitly/
- RightCrowd, identidad única para visitas, contratistas y empleados: https://www.rightcrowd.com/solutions/visitor-management/
- Formatos de identificación de Costa Rica (cédula, DIMEX): https://lookuptax.com/docs/tax-identification-number/costa-rica-tax-id-guide
- DIMEX, Correos de Costa Rica: https://correos.go.cr/cedula-residencia/
- CWE-1289: https://cwe.mitre.org/data/definitions/1289.html
