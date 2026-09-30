package com.brisas.controlacceso

import java.math.BigInteger
import java.util.Base64
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CodigoVinculacionTest {
    @Test
    fun `normaliza a mayusculas y sin separadores`() {
        assertEquals("K7QMR4XT2P", CodigoVinculacion.normalizar(" k7qm-r4xt 2p "))
    }

    @Test
    fun `agrupa 4-4-2 y corta al largo del codigo`() {
        assertEquals("K7", CodigoVinculacion.formatear("k7"))
        assertEquals("K7QM-R", CodigoVinculacion.formatear("k7qmr"))
        assertEquals("K7QM-R4XT-2P", CodigoVinculacion.formatear("K7QMR4XT2PZZZ"))
    }

    @Test
    fun `solo esta completo con los 10 caracteres`() {
        assertFalse(CodigoVinculacion.completo("K7QM-R4XT"))
        assertTrue(CodigoVinculacion.completo("K7QM-R4XT-2P"))
    }

    @Test
    fun `lee el codigo del QR del panel`() {
        assertEquals("K7QMR4XT2P", CodigoVinculacion.desdeQr("brisas-acceso://vincular?codigo=K7QMR4XT2P"))
        assertEquals("K7QMR4XT2P", CodigoVinculacion.desdeQr("K7QM-R4XT-2P"))
    }

    @Test
    fun `descarta QR que no son de vinculacion`() {
        assertNull(CodigoVinculacion.desdeQr("https://ejemplo.com"))
        assertNull(CodigoVinculacion.desdeQr("brisas-acceso://otra?codigo=K7QMR4XT2P"))
        assertNull(CodigoVinculacion.desdeQr("brisas-acceso://vincular?codigo=CORTO"))
    }

    @Test
    fun `el JWK usa coordenadas de 32 bytes en base64url`() {
        // Clave pública del ejemplo de RFC 7517, apéndice A.1.
        val decodificar = { texto: String -> BigInteger(1, Base64.getUrlDecoder().decode(texto)) }
        val x = "f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU"
        val y = "x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0"
        assertEquals(
            """{"kty":"EC","crv":"P-256","x":"$x","y":"$y"}""",
            jwkP256(decodificar(x), decodificar(y)),
        )
    }

    @Test
    fun `el JWK rellena coordenadas cortas con ceros a la izquierda`() {
        val jwk = jwkP256(BigInteger.ONE, BigInteger.TWO)
        val x = Regex("\"x\":\"([^\"]+)\"").find(jwk)!!.groupValues[1]
        val bytes = Base64.getUrlDecoder().decode(x)
        assertEquals(32, bytes.size)
        assertEquals(1, bytes.last().toInt())
    }
}
