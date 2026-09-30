package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ExpulsionNubeTest {
    private fun motivo(dispositivo: String?, motivo: String?, huella: String?) =
        MotivoExpulsion.paraEsteEquipo(dispositivo, motivo, huella, dispositivoId = "d1", huella = "h1")

    @Test
    fun `ignora avisos de otro dispositivo del sitio`() {
        assertNull(motivo("d2", "revocado", null))
    }

    @Test
    fun `aplica revocacion y suspension de este dispositivo`() {
        assertEquals(MotivoExpulsion.REVOCADO, motivo("d1", "revocado", "h1"))
        assertEquals(MotivoExpulsion.SUSPENDIDO, motivo("d1", "suspendido", null))
    }

    @Test
    fun `al re-vincular solo expulsa al equipo cuya huella quedo fuera`() {
        assertEquals(MotivoExpulsion.REVINCULADO, motivo("d1", "revinculado", "h1"))
        assertNull(motivo("d1", "revinculado", "h-vieja"))
    }

    @Test
    fun `descarta motivos desconocidos`() {
        assertNull(motivo("d1", "otro", "h1"))
        assertNull(motivo("d1", null, "h1"))
    }
}
