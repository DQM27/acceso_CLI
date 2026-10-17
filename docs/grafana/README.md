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

## Mantenerlos al día

El tablero que manda es el de Grafana. Después de un cambio importante,
exportarlo (**Share → Export → Export as JSON**) y reemplazar el archivo. Una
alerta se exporta desde su menú (**Export**). Los JSON no llevan datos ni
credenciales: sólo consultas y diseño.
