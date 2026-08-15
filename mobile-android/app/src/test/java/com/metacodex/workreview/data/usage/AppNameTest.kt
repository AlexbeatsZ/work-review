package com.metacodex.workreview.data.usage

import org.junit.Assert.assertEquals
import org.junit.Test

class AppNameTest {
    @Test
    fun usesResolvedLabelWhenItIsHumanReadable() {
        assertEquals("Claude", resolveAppDisplayName("com.anthropic.claude", "Claude"))
    }

    @Test
    fun replacesPackageFallbackWithReadableLastSegment() {
        assertEquals("Notes", resolveAppDisplayName("com.example.notes", "com.example.notes"))
    }

    @Test
    fun preservesKnownLocalizedNames() {
        assertEquals("微信", resolveAppDisplayName("com.tencent.mm", null))
    }
}
