-- Usuario ROOT de acceso rápido para el piloto móvil — cédula 123456789,
-- contraseña "clave_prueba_123" (valor de prueba, sin relación con ninguna
-- contraseña real). Hash real de Argon2 (mismo esquema que
-- services::password::generar_hash), no texto plano. Cambiar antes de
-- sembrar cualquier base que no sea puramente de desarrollo.
INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
VALUES (
    '123456789',
    'Root Piloto',
    '$argon2id$v=19$m=19456,t=2,p=1$pO+/qvY8ieaUA97ME2LUPQ$OfE/070ufOj4TtL2SzVyW3sefnJjrMJq32APEHrM/wI',
    'ROOT',
    1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    password_hash = excluded.password_hash,
    rol = excluded.rol,
    activo = excluded.activo;
