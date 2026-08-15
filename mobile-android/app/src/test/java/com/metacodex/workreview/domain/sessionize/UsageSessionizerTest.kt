package com.metacodex.workreview.domain.sessionize

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.LocalDate
import java.time.ZoneId

class UsageSessionizerTest {
    private val zone = ZoneId.of("Asia/Shanghai")
    private val labels = object : AppLabelResolver {
        override fun labelFor(packageName: String): String = "Label:$packageName"
    }

    @Test
    fun appSwitchClosesPreviousSession() {
        val base = LocalDate.of(2026, 6, 2).atStartOfDay(zone).toInstant().toEpochMilli()
        val sessions = UsageSessionizer(zone, minSessionMs = 0).sessionize(
            events = listOf(
                UsageEventModel(base + 1_000, "a", null, UsageSessionizer.EVENT_ACTIVITY_RESUMED),
                UsageEventModel(base + 6_000, "b", null, UsageSessionizer.EVENT_ACTIVITY_RESUMED),
                UsageEventModel(base + 9_000, "b", null, UsageSessionizer.EVENT_ACTIVITY_PAUSED)
            ),
            rangeStart = base,
            rangeEnd = base + 10_000,
            labelResolver = labels
        )
        assertEquals(2, sessions.size)
        assertEquals("a", sessions[0].packageName)
        assertEquals(5_000, sessions[0].durationMs)
        assertEquals("b", sessions[1].packageName)
        assertEquals(3_000, sessions[1].durationMs)
    }

    @Test
    fun splitsAcrossMidnight() {
        val start = LocalDate.of(2026, 6, 2).atTime(23, 59).atZone(zone).toInstant().toEpochMilli()
        val end = LocalDate.of(2026, 6, 3).atTime(0, 1).atZone(zone).toInstant().toEpochMilli()
        val sessions = UsageSessionizer(zone, minSessionMs = 0).sessionize(
            events = listOf(
                UsageEventModel(start, "via", null, UsageSessionizer.EVENT_ACTIVITY_RESUMED),
                UsageEventModel(end, "via", null, UsageSessionizer.EVENT_ACTIVITY_PAUSED)
            ),
            rangeStart = start,
            rangeEnd = end,
            labelResolver = labels
        )
        assertEquals(2, sessions.size)
        assertTrue(sessions.all { it.durationMs == 60_000L })
    }

    @Test
    fun ignoresZeroDurationSessions() {
        val base = LocalDate.of(2026, 6, 2).atStartOfDay(zone).toInstant().toEpochMilli()
        val sessions = UsageSessionizer(zone, minSessionMs = 0).sessionize(
            events = listOf(
                UsageEventModel(base, "a", null, UsageSessionizer.EVENT_ACTIVITY_RESUMED),
                UsageEventModel(base, "a", null, UsageSessionizer.EVENT_ACTIVITY_PAUSED)
            ),
            rangeStart = base,
            rangeEnd = base + 1_000,
            labelResolver = labels
        )
        assertTrue(sessions.isEmpty())
    }

    @Test
    fun filtersSessionsBelowMinimumDuration() {
        val base = LocalDate.of(2026, 6, 2).atStartOfDay(zone).toInstant().toEpochMilli()
        val sessions = UsageSessionizer(zone, minSessionMs = 60_000).sessionize(
            events = listOf(
                UsageEventModel(base, "a", null, UsageSessionizer.EVENT_ACTIVITY_RESUMED),
                UsageEventModel(base + 10_000, "a", null, UsageSessionizer.EVENT_ACTIVITY_PAUSED)
            ),
            rangeStart = base,
            rangeEnd = base + 20_000,
            labelResolver = labels
        )
        assertTrue(sessions.isEmpty())
    }

    @Test
    fun ignoresConfiguredPackages() {
        val base = LocalDate.of(2026, 6, 2).atStartOfDay(zone).toInstant().toEpochMilli()
        val sessions = UsageSessionizer(
            zoneId = zone,
            minSessionMs = 0,
            ignoredPackages = setOf("blocked.app")
        ).sessionize(
            events = listOf(
                UsageEventModel(base, "blocked.app", null, UsageSessionizer.EVENT_ACTIVITY_RESUMED),
                UsageEventModel(base + 120_000, "blocked.app", null, UsageSessionizer.EVENT_ACTIVITY_PAUSED),
                UsageEventModel(base + 130_000, "allowed.app", null, UsageSessionizer.EVENT_ACTIVITY_RESUMED),
                UsageEventModel(base + 190_000, "allowed.app", null, UsageSessionizer.EVENT_ACTIVITY_PAUSED)
            ),
            rangeStart = base,
            rangeEnd = base + 200_000,
            labelResolver = labels
        )
        assertEquals(1, sessions.size)
        assertEquals("allowed.app", sessions.single().packageName)
    }
}
