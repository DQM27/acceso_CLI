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

## Importar

En Grafana: **Dashboards → New → Import → Upload JSON file**. Al importar,
elegir el origen de datos PostgreSQL del staging. Los archivos traen el `uid`
del origen de datos actual (`eg1fe86yip534f`); si en otra cuenta tiene otro
`uid`, reemplazarlo en el JSON antes de importar.

## Mantenerlos al día

El tablero que manda es el de Grafana. Después de un cambio importante,
exportarlo (**Share → Export → Export as JSON**) y reemplazar el archivo.
Los JSON no llevan datos ni credenciales: sólo consultas y diseño.
