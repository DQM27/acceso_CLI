# Integración de la web de visitas

Fecha: 2026-09-09. Aplicación independiente en `web-visitas/`; el panel conserva `web/`.
Este documento complementa `handoff-web-anfitriones.md`. No aplica cambios a Postgres.

## Bloqueo confirmado en el backend

En el proyecto `xidaepyaljzkpbsxrqsm`, una consulta de solo lectura con rol
`authenticated` a `anfitriones` devuelve `42P17: infinite recursion detected in
policy for relation "citas"`. Reproducción sin crear usuarios ni modificar datos:

```sql
begin read only;
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"00000000-0000-4000-8000-000000000099","email":"revision-web@example.invalid","role":"authenticated"}', true);
select count(*) from public.anfitriones where correo = 'revision-web@example.invalid';
rollback;
```

Las políticas de `citas` consultan `cita_sitios` y viceversa; la nueva política
de lectura de `anfitriones` también llega a ese ciclo. El encargado del backend
debe resolverlo conservando el aislamiento entre anfitriones, dispositivos y
administradores. Verificar con roles reales; una consulta como `postgres` no
comprueba RLS.

## Guardado atómico requerido

Tres peticiones REST independientes no forman una transacción. Una interrupción
puede dejar una autorización vigente sin todos sus visitantes o sitios. La web
usará una única RPC propuesta, pendiente de implementar y coordinar:

`public.crear_cita_anfitrion(p_id uuid, p_fecha_desde date, p_fecha_hasta date,
p_motivo text, p_sitios uuid[], p_visitantes jsonb) returns uuid`

Cada visitante contiene `cedula`, `nombre`, `empresa` y `placa_vehiculo`; los dos
últimos admiten null. El correo se obtiene de la identidad autenticada en el
servidor; nunca se acepta un correo de propietario enviado por el navegador.

Requisitos del servidor:

- Crear cabecera, sitios y visitantes en una misma transacción.
- Exigir un anfitrión autorizado en cada operación y mantener RLS; preferir
  `SECURITY INVOKER`. Denegar ejecución anónima.
- Validar fechas reales, inicio no anterior al día actual en `America/Costa_Rica`,
  fin mayor o igual al inicio, 1–50 visitantes y 1–100 sitios existentes y únicos.
- Límites de texto: nombre 150, documento 30, empresa 150, placa 20 y motivo 1000.
  Nombre y documento son obligatorios. Rechazar caracteres de control.
- Normalizar el documento de acuerdo con el lector del núcleo y rechazar
  duplicados dentro del grupo. La web conserva letras/números, convierte a
  mayúsculas y elimina espacios y guiones; confirmar este contrato con el núcleo.
- `p_id` es la clave de idempotencia generada antes del primer envío. Un reintento
  con el mismo propietario y contenido devuelve el mismo UUID sin duplicar filas;
  si cambió el contenido o pertenece a otra cuenta, rechazarlo. Comparar contenido
  normalizado; no devolver datos de otro anfitrión.
- Resolver primero un UUID idempotente ya confirmado, antes de rechazar fechas
  pasadas: una respuesta perdida puede reintentarse después de medianoche. La
  validación del día actual aplica a una solicitud nueva.
- Aplicar límites de frecuencia/cuota del lado servidor: los controles de la web
  son de usabilidad, cualquiera con un token puede llamar directamente la API.
- Si se mantienen permisos de escritura directa sobre tablas, validar también
  ese camino con restricciones y políticas; no limitar la validación a la RPC.
- Resolver la baja de anfitriones conservando sus citas: la FK impide eliminar
  físicamente una fila que tenga historial. La revocación debe afectar operaciones
  con tokens que todavía no hayan vencido.
- Cancelar con `UPDATE estado = 'CANCELADA'` sobre cita propia, sin DELETE.
  Impedir reactivación de citas canceladas si se mantiene el contrato de cancelación
  definitiva; comprobarlo también con peticiones directas.

La web no sustituye esta RPC por inserciones parciales. Si falta, informa que no
pudo guardar y conserva el formulario. La lectura usa las tablas y relaciones del
handoff; la cancelación exige que el UPDATE devuelva exactamente la cita afectada.

## Verificación compartida antes de publicar

Dos anfitriones de prueba: cada uno lee/cancela solo sus citas; una cuenta sin alta
no lee ni escribe; un dispositivo ve solo los sitios autorizados. Probar rollback
con un sitio inválido, visitante inválido y duplicados, reintento tras respuesta
perdida, cancelación concurrente y revocación del anfitrión con un token aún vigente.
No utilizar personas ni citas reales en estas pruebas.

Dominio padre confirmado por el usuario: `megabrisas.com`. La aplicación utiliza
`visitas.megabrisas.com`. Añadir solo la URL exacta de retorno OAuth en
Supabase; no cambiar la URL principal del panel para hacer funcionar esta app.
