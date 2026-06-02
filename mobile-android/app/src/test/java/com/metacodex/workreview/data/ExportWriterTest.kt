package com.metacodex.workreview.data

import org.junit.Assert.assertEquals
import org.junit.Test

class ExportWriterTest {
    @Test
    fun exportFileNamesAreStable() {
        assertEquals("activities.csv", ExportWriter.ACTIVITIES_CSV)
        assertEquals("app_sessions.csv", ExportWriter.APP_SESSIONS_CSV)
        assertEquals("work_review_mobile.db", ExportWriter.DB_FILE)
    }
}
