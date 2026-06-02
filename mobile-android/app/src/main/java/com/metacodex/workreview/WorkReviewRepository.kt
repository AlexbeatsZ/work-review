package com.metacodex.workreview

import android.content.Context
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import com.metacodex.workreview.data.db.ActivityEntity
import com.metacodex.workreview.data.db.AppSessionEntity
import com.metacodex.workreview.data.db.UserPreferenceEntity
import com.metacodex.workreview.data.db.WorkReviewDatabase
import com.metacodex.workreview.data.export.ExportSchedule
import com.metacodex.workreview.data.usage.InstalledApp
import com.metacodex.workreview.data.usage.UsageStatsCollector
import com.metacodex.workreview.permissions.UsageAccess
import java.time.LocalDate
import java.time.LocalTime
import java.time.ZoneId
import java.time.ZonedDateTime
import java.util.concurrent.TimeUnit

class WorkReviewRepository(private val context: Context) {
    private val db = WorkReviewDatabase.get(context)
    private val dao = db.dao()
    private val zone = ZoneId.systemDefault()

    fun usageSummary(date: LocalDate) = dao.usageSummaryForDay(date.startMs(), date.endMs())
    fun sessions(date: LocalDate) = dao.sessionsForDay(date.startMs(), date.endMs())
    fun recentAppEvents(limit: Int = 20) = dao.recentAppEvents(limit)
    fun recentAppSessions(limit: Int = 20) = dao.recentAppSessions(limit)

    suspend fun collectUsageNow() {
        if (!UsageAccess.hasUsageAccess(context)) return
        val now = System.currentTimeMillis()
        val fallbackStart = LocalDate.now(zone).startMs()
        val last = dao.getPreference(KEY_LAST_COLLECTED_TS)?.toLongOrNull() ?: fallbackStart
        val start = last.coerceAtMost(now)
        val collector = UsageStatsCollector(
            context = context,
            minSessionMs = minSessionMs(),
            mergeGapMs = mergeGapMs(),
            ignoredPackages = ignoredPackages()
        )
        val result = collector.collect(start, now)
        dao.insertAppEvents(result.events)
        dao.deleteUsageSessionsBetween(start, now)
        dao.insertAppSessions(result.sessions)
        dao.insertActivities(result.sessions.map { it.toActivity() })
        dao.putPreference(UserPreferenceEntity(KEY_LAST_COLLECTED_TS, now.toString()))
        dao.putPreference(UserPreferenceEntity(KEY_LAST_COLLECTION_AT, now.toString()))
    }

    suspend fun recollectUsageForDate(date: LocalDate) {
        if (!UsageAccess.hasUsageAccess(context)) return
        val start = date.startMs()
        val end = minOf(date.endMs(), System.currentTimeMillis())
        val collector = UsageStatsCollector(
            context = context,
            minSessionMs = minSessionMs(),
            mergeGapMs = mergeGapMs(),
            ignoredPackages = ignoredPackages()
        )
        val result = collector.collect(start, end)
        dao.insertAppEvents(result.events)
        dao.deleteUsageSessionsBetween(start, end)
        dao.insertAppSessions(result.sessions)
        dao.deleteUsageActivitiesBetween(start / 1000, end / 1000)
        dao.insertActivities(result.sessions.map { it.toActivity() })
    }

    suspend fun clearAppUsage() {
        dao.clearAppEvents()
        dao.clearAppSessions()
        dao.clearUsageActivities()
    }

    suspend fun deleteAppSession(id: Long) {
        dao.deleteAppSession(id)
    }

    suspend fun setMinSessionSeconds(seconds: Int) {
        dao.putPreference(UserPreferenceEntity(KEY_MIN_SESSION_SECONDS, seconds.coerceAtLeast(1).toString()))
    }

    suspend fun minSessionSeconds(): Int =
        dao.getPreference(KEY_MIN_SESSION_SECONDS)?.toIntOrNull() ?: 60

    private suspend fun minSessionMs(): Long = minSessionSeconds() * 1000L

    private suspend fun mergeGapMs(): Long =
        (dao.getPreference(KEY_MERGE_GAP_SECONDS)?.toIntOrNull() ?: 15) * 1000L

    suspend fun setIgnoredPackages(value: String) {
        dao.putPreference(UserPreferenceEntity(KEY_IGNORED_PACKAGES, value))
    }

    suspend fun setIgnoredPackages(packages: Set<String>) {
        setIgnoredPackages(packages.sorted().joinToString("\n"))
    }

    suspend fun ignoredPackagesText(): String =
        dao.getPreference(KEY_IGNORED_PACKAGES) ?: ""

    private suspend fun ignoredPackages(): Set<String> =
        ignoredPackagesText()
            .split(',', '\n', ';')
            .map { it.trim() }
            .filter { it.isNotBlank() }
            .toSet()

    suspend fun installedApps(): List<InstalledApp> {
        val ignored = ignoredPackages()
        return context.packageManager.getInstalledApplications(0)
            .map { info ->
                InstalledApp(
                    packageName = info.packageName,
                    label = runCatching { context.packageManager.getApplicationLabel(info).toString() }
                        .getOrDefault(info.packageName),
                    isIgnored = ignored.contains(info.packageName)
                )
            }
            .sortedWith(compareBy<InstalledApp> { !it.isIgnored }.thenBy { it.label.lowercase() })
    }

    fun schedulePeriodicCollection() {
        val request = PeriodicWorkRequestBuilder<UsageCollectionWorker>(15, TimeUnit.MINUTES)
            .build()
        WorkManager.getInstance(context).enqueueUniquePeriodicWork(
            "usage-collection",
            ExistingPeriodicWorkPolicy.UPDATE,
            request
        )
    }

    suspend fun scheduleTimedAutoExports() {
        val workManager = WorkManager.getInstance(context)
        exportTimes().forEachIndexed { index, time ->
            workManager.enqueueUniqueWork(
                "timed-auto-export-$index",
                ExistingWorkPolicy.REPLACE,
                androidx.work.OneTimeWorkRequestBuilder<AutoExportWorker>()
                    .setInitialDelay(nextExportDelayMs(time), TimeUnit.MILLISECONDS)
                    .addTag("timed-auto-export")
                    .build()
            )
        }
    }

    suspend fun rescheduleTimedAutoExports() {
        WorkManager.getInstance(context).cancelAllWorkByTag("timed-auto-export")
        scheduleTimedAutoExports()
    }

    suspend fun setAutoExportEnabled(enabled: Boolean) {
        dao.putPreference(UserPreferenceEntity(KEY_AUTO_EXPORT_ENABLED, enabled.toString()))
        if (enabled) {
            rescheduleTimedAutoExports()
        } else {
            WorkManager.getInstance(context).cancelAllWorkByTag("timed-auto-export")
        }
    }

    suspend fun autoExportEnabled(): Boolean =
        dao.getPreference(KEY_AUTO_EXPORT_ENABLED)?.toBooleanStrictOrNull() ?: true

    suspend fun setAutoExportTime(value: String) {
        setAutoExportTimes(value)
    }

    suspend fun autoExportTime(): String =
        autoExportTimesText()

    suspend fun setAutoExportTimes(value: String) {
        dao.putPreference(UserPreferenceEntity(KEY_AUTO_EXPORT_TIME, ExportSchedule.parseTimes(value).joinToString("\n")))
        if (autoExportEnabled()) rescheduleTimedAutoExports()
    }

    suspend fun autoExportTimesText(): String =
        dao.getPreference(KEY_AUTO_EXPORT_TIME) ?: ExportSchedule.DEFAULT_TIME

    suspend fun exportTimes(): List<String> = ExportSchedule.parseTimes(autoExportTimesText())

    private fun nextExportDelayMs(timeText: String): Long {
        val target = runCatching { LocalTime.parse(ExportSchedule.normalizeTime(timeText)) }
            .getOrDefault(LocalTime.of(11, 30))
        val now = ZonedDateTime.now(zone)
        var next = now.toLocalDate().atTime(target).atZone(zone)
        if (!next.isAfter(now)) next = next.plusDays(1)
        return java.time.Duration.between(now, next).toMillis().coerceAtLeast(60_000)
    }

    suspend fun setExportDirectoryUri(uri: String?) {
        dao.putPreference(UserPreferenceEntity(KEY_EXPORT_DIRECTORY_URI, uri.orEmpty()))
    }

    suspend fun exportDirectoryUri(): String? =
        dao.getPreference(KEY_EXPORT_DIRECTORY_URI)?.takeIf { it.isNotBlank() }

    suspend fun setLastExportAt(timestamp: Long) {
        dao.putPreference(UserPreferenceEntity(KEY_LAST_EXPORT_AT, timestamp.toString()))
    }

    suspend fun lastExportAt(): Long? =
        dao.getPreference(KEY_LAST_EXPORT_AT)?.toLongOrNull()

    suspend fun lastCollectionAt(): Long? =
        dao.getPreference(KEY_LAST_COLLECTION_AT)?.toLongOrNull()

    private fun AppSessionEntity.toActivity(): ActivityEntity =
        ActivityEntity(
            timestamp = endTs / 1000,
            appName = appLabel ?: packageName,
            windowTitle = appLabel ?: packageName,
            category = category,
            duration = durationMs / 1000,
            executablePath = packageName,
            semanticCategory = semanticCategory,
            semanticConfidence = semanticConfidence,
            mobileSource = "android_usage"
        )

    private fun LocalDate.startMs(): Long =
        atStartOfDay(zone).toInstant().toEpochMilli()

    private fun LocalDate.endMs(): Long = plusDays(1).startMs()

    companion object {
        private const val KEY_LAST_COLLECTED_TS = "last_collected_ts"
        private const val KEY_MIN_SESSION_SECONDS = "min_session_seconds"
        private const val KEY_MERGE_GAP_SECONDS = "merge_gap_seconds"
        private const val KEY_AUTO_EXPORT_ENABLED = "auto_export_enabled"
        private const val KEY_AUTO_EXPORT_TIME = "auto_export_time"
        private const val KEY_IGNORED_PACKAGES = "ignored_packages"
        private const val KEY_EXPORT_DIRECTORY_URI = "export_directory_uri"
        private const val KEY_LAST_EXPORT_AT = "last_export_at"
        private const val KEY_LAST_COLLECTION_AT = "last_collection_at"
    }
}
