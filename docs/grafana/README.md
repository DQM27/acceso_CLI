# Tableros de Grafana

Copia de los tableros que viven en Grafana Cloud, para poder recrearlos si se
pierden o se cambia de cuenta. Los dos leen del **staging** de Supabase con el
usuario de sólo lectura `grafana_lector`
(`supabase/scripts/telemetria_lector_grafana_staging.sql` y
`supabase/scripts/errores_app_staging.sql`).

| Archivo | Tablero | Tablas |
|---|---|---|
| `lattis-telemetria.json` | Lattis · Telemetría (staging) | `telemetria_diagnostico` |
| `lattis-errores.json` | Lattis · Errores (staging) | `errores_app`, `errores_agrupados` |

Alertas (carpeta `Lattis · Alertas`, aviso por correo al punto de contacto
`correo-daniel`):

| Archivo | Regla | Cuándo avisa |
|---|---|---|
| `alertas/equipo-nuevo-en-la-pantalla-de-vinculacion.json` | Equipo nuevo en la pantalla de vinculación | Un teléfono llegó a la pantalla de primer arranque o al escáner del código, en los últimos 15 minutos |
| `alertas/error-nuevo-en-las-apps.json` | Error nuevo en las apps | Apareció por primera vez, en los últimos 15 minutos, un error (huella) que no se había visto; una instancia por error, con su plataforma y título |

## Importar

En Grafana: **Dashboards → New → Import → Upload JSON file**. Al importar,
elegir el origen de datos PostgreSQL del staging. Los archivos traen el `uid`
del origen de datos actual (`eg1fe86yip534f`); si en otra cuenta tiene otro
`uid`, reemplazarlo en el JSON antes de importar.

### Importar una alerta

Cada archivo es una regla en el formato de la API de aprovisionamiento. Con un
token de cuenta de servicio:

```
curl -X POST "$GRAFANA_URL/api/v1/provisioning/alert-rules" \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -H "X-Disable-Provenance: true" \
  --data @docs/grafana/alertas/error-nuevo-en-las-apps.json
```

Antes tiene que existir la carpeta (`lattis-alertas`) y el punto de contacto
(`correo-daniel`); los `uid` de la carpeta y del origen de datos son los de la
cuenta actual.

### Texto de las alertas

Los avisos (`summary` y `description`) llevan sólo texto y plantillas simples
(`{{ $labels.plataforma }}`). Una versión anterior de «Error nuevo en las apps»
traía en la descripción paréntesis y una sentencia SQL (`update ... where huella
= '{{ $labels.huella }}';`), y la pantalla de alertas de Grafana mostraba
`1:1: parse error: unexpected "("` cada vez que se abría o se recargaba. Al
dejar el texto simple desapareció; no se aisló si fueron los paréntesis o el
SQL, así que conviene no repetir ninguno de los dos. El `update` para marcar un
error como resuelto está en la descripción del panel «Errores agrupados».

### Conexiones a Supabase

El usuario `grafana_lector` tiene un límite de **5 conexiones**
(`supabase/scripts/telemetria_lector_grafana_staging.sql`). El origen de datos
de Grafana tiene que quedar por debajo: **máximo de conexiones abiertas = 4**
(Connections → Data sources → PostgreSQL → Connection limits). Con el valor por
defecto (100), varios paneles y alertas a la vez pasaban el límite y la base
respondía `too many connections for role "grafana_lector"` (SQLSTATE 53300).

Ese error ya provocó una falsa alarma: la regla de seguridad «Equipo nuevo en la
pantalla de vinculación» estaba configurada con `execErrState: Alerting`, así que
una consulta fallida mandaba el mismo correo que un equipo nuevo real. Ahora esa
regla usa `execErrState: Error` (un fallo de la base sale como `DatasourceError`,
con su propio nombre) y `for: 2m` (un error de una sola evaluación no avisa; un
equipo real sigue avisando, porque cuenta durante 15 minutos).

## Mantenerlos al día

El tablero que manda es el de Grafana. Después de un cambio importante,
exportarlo (**Share → Export → Export as JSON**) y reemplazar el archivo. Una
alerta se exporta desde su menú (**Export**). Los JSON no llevan datos ni
credenciales: sólo consultas y diseño.
