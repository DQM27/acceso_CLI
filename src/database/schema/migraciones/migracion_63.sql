-- Orden de telemetría de ESTE equipo (plan de telemetría por estudios,
-- docs/features-futuras/plan-telemetria-y-registro-de-errores.md, fase 2): qué
-- grupos de eventos mide, hasta cuándo y con qué versión de la orden.
--
-- Una sola fila (`id = 1`). La escribe la sincronización o el aviso en vivo
-- `telemetria_config` cuando llega una orden NUEVA (versión mayor), y se lee
-- una vez al abrir el núcleo: cada medición consulta una copia en memoria
-- (`telemetria::config`), nunca esta tabla. Sin fila, el equipo mide `base` y
-- `rendimiento`.
--
-- `dispositivo_id` es el equipo al que iba la orden: si el equipo se vincula de
-- nuevo con otra identidad, una orden de la identidad anterior no se aplica.
-- `grupos` son los nombres separados por coma, sin `base` (que va siempre);
-- vacío = ningún grupo además de `base`. `vence_en` en UTC al segundo, o NULL
-- si no vence; pasado ese instante el equipo vuelve al estado por defecto
-- aunque no tenga red.
CREATE TABLE config_telemetria (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    dispositivo_id TEXT NOT NULL CHECK (length(dispositivo_id) BETWEEN 1 AND 80),
    version INTEGER NOT NULL CHECK (version >= 1),
    grupos TEXT NOT NULL,
    vence_en TEXT CHECK (
        vence_en IS NULL
        OR strftime('%Y-%m-%dT%H:%M:%SZ', vence_en) IS vence_en
    ),
    recibida_en TEXT NOT NULL
) STRICT;
