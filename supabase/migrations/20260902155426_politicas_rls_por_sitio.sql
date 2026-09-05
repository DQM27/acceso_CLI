-- Cada dispositivo autenticado (JWT con claim sitio_id) solo ve y toca
-- datos de su propio sitio. Nada de esto aplica a service_role (nosotros
-- por MCP/administración), que sigue con acceso total como hasta ahora.

-- dispositivos: un dispositivo puede leer (no escribir) los dispositivos
-- de su propio sitio -- para mostrar "cerrado por PC/celular" en pantalla.
create policy "leer dispositivos del propio sitio"
on dispositivos for select
to authenticated
using (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid);

-- contratistas (espejo): leer, crear y actualizar solo dentro del sitio.
create policy "leer contratistas del propio sitio"
on contratistas for select
to authenticated
using (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid);

create policy "crear contratistas del propio sitio"
on contratistas for insert
to authenticated
with check (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid);

create policy "actualizar contratistas del propio sitio"
on contratistas for update
to authenticated
using (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid)
with check (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid);

-- ingresos (cola): mismo criterio. El "primero en llegar gana" para
-- cerrar un ingreso se aplica aparte, con un WHERE version=... en el
-- UPDATE -- RLS acá solo garantiza que nunca se toque un ingreso de
-- otro sitio.
create policy "leer ingresos del propio sitio"
on ingresos for select
to authenticated
using (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid);

create policy "crear ingresos del propio sitio"
on ingresos for insert
to authenticated
with check (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid);

create policy "actualizar ingresos del propio sitio"
on ingresos for update
to authenticated
using (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid)
with check (sitio_id = (auth.jwt() ->> 'sitio_id')::uuid);
