-- Usuario ROOT de acceso rápido para el piloto móvil — cédula 123456789,
-- contraseña "daniel27". Hash real de Argon2 (mismo esquema que
-- services::password::generar_hash), no texto plano.
INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
VALUES (
    '123456789',
    'Root Piloto',
    '$argon2id$v=19$m=19456,t=2,p=1$FZShq0MtV2bGh9nFBgvrGA$dYNDyh7up/wmAY+t/Vf6V5LTCS9sNkQgaH81G650xfM',
    'ROOT',
    1
)
ON CONFLICT(cedula) DO UPDATE SET
    nombre = excluded.nombre,
    password_hash = excluded.password_hash,
    rol = excluded.rol,
    activo = excluded.activo;
