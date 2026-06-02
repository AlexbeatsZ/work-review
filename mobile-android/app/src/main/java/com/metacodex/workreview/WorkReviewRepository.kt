package com.metacodex.workreview

import android.content.Context
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import com.metacodex.workreview.data.browser.PrivacyFilter
import com.metacodex.workreview.data.browser.PrivacySettings
import com.metacodex.workreview.data.db.ActivityEntity
import com.metacodex.workreview.data.db.AppSessionEntity
import com.metacodex.workreview.data.db.BrowserEventEntity
import com.metacodex.workreview.data.db.UserPreferenceEntity
import com.metacodex.workreview.data.db.WorkReviewDatabase
import com.metacodex.workreview.data.tagging.AutoTagger
import com.metacodex.workreview.data.usage.UsageStatsCollector
import com.metacodex.workreview.permissions.UsageAccess
import kotlinx.coroutines.flow.Flow
import java.time.LocalDate
import java.time.ZoneId
import java.util.concurrent.TimeUnit

class WorkReviewRepository(private val context: Context) {
    private val db = WorkReviewDatabase.get(context)
    private val dao = db.dao()
    private val zone = ZoneId.systemDefault()

    fun usageSummary(date: LocalDate) = dao.usageSummaryForDay(date.startMs(), date.endMs())
    fun sessions(date: LocalDate) = dao.sessionsForDay(date.startMs(), date.endMs())
    fun browserEvents(date: LocalDate): Flow<List<BrowserEventEntity>> =
        dao.browserEventsForDay(date.startMs(), date.endMs())
    fun recentAppEvents(limit: Int = 20) = dao.recentAppEvents(limit)
    fun recentAppSessions(limit: Int = 20) = dao.recentAppSessions(limit)
    fun recentBrowserEvents(limit: Int = 20) = dao.recentBrowserEvents(limit)

    suspend fun collectUsageNow() {
        if (!UsageAccess.hasUsageAccess(context)) return
        val now = System.currentTimeMillis()
        val fallbackStart = LocalDate.now(zone).startMs()
        val last = dao.getPreference(KEY_LAST_COLLECTED_TS)?.toLongOrNull() ?: fallbackStart
        val start = last.coerceAtMost(now)
        val collector = UsageStatsCollector(
            context = context,
            minSessionMs = minSessionMs(),
            mergeGapMs = mergeGapMs()
        )
        val result = collector.collect(start, now)
        dao.insertAppEvents(result.events)
        dao.deleteUsageSessionsBetween(start, now)
        dao.insertAppSessions(result.sessions)
        dao.insertActivities(result.sessions.map { it.toActivity() })
        dao.putPreference(UserPreferenceEntity(KEY_LAST_COLLECTED_TS, now.toString()))
    }

    suspend fun recollectUsageForDate(date: LocalDate) {
        if (!UsageAccess.hasUsageAccess(context)) return
        val start = date.startMs()
        val end = minOf(date.endMs(), System.currentTimeMillis())
        val collector = UsageStatsCollector(
            context = context,
            minSessionMs = minSessionMs(),
            mergeGapMs = mergeGapMs()
        )
        val result = collector.collect(start, end)
        dao.insertAppEvents(result.events)
        dao.deleteUsageSessionsBetween(start, end)
        dao.insertAppSessions(result.sessions)
        dao.deleteUsageActivitiesBetween(start / 1000, end / 1000)
        dao.insertActivities(result.sessions.map { it.toActivity() })
    }

    suspend fun insertBrowserEvent(event: BrowserEventEntity) {
        val settings = browserPrivacySettings()
        PrivacyFilter(settings).apply(event)?.let { filtered ->
            val classification = AutoTagger.classifyUrl(filtered.url, filtered.title)
            val viaSession = findViaSessionFor(filtered.ts)
            val enriched = filtered.copy(
                viaSessionId = viaSession?.id,
                category = classification.category,
                semanticCategory = classification.semanticCategory,
                semanticConfidence = classification.semanticConfidence
            )
            dao.insertBrowserEvent(enriched)
            dao.insertActivities(listOf(enriched.toActivity(viaSession)))
        }
    }

    suspend fun setLocalServerEnabled(enabled: Boolean) {
        dao.putPreference(UserPreferenceEntity(KEY_LOCAL_SERVER_ENABLED, enabled.toString()))
    }

    suspend fun localServerEnabled(): Boolean =
        dao.getPreference(KEY_LOCAL_SERVER_ENABLED)?.toBooleanStrictOrNull() ?: false

    suspend fun setBrowserLoggingEnabled(enabled: Boolean) {
        dao.putPreference(UserPreferenceEntity(KEY_BROWSER_LOGGING_ENABLED, enabled.toString()))
    }

    suspend fun setDomainOnly(enabled: Boolean) {
        dao.putPreference(UserPreferenceEntity(KEY_DOMAIN_ONLY, enabled.toString()))
    }

    suspend fun setBlockedDomains(domains: String) {
        dao.putPreference(UserPreferenceEntity(KEY_BLOCKED_DOMAINS, domains))
    }

    suspend fun browserPrivacySettings(): PrivacySettings =
        PrivacySettings(
            browserLoggingEnabled = dao.getPreference(KEY_BROWSER_LOGGING_ENABLED)
                ?.toBooleanStrictOrNull() ?: true,
            domainOnly = dao.getPreference(KEY_DOMAIN_ONLY)?.toBooleanStrictOrNull() ?: false,
            blockedDomains = dao.getPreference(KEY_BLOCKED_DOMAINS)
                ?.split(',', '\n')
                ?.map { it.trim().lowercase() }
                ?.filter { it.isNotBlank() }
                ?.toSet()
                ?: emptySet()
        )

    suspend fun clearAppUsage() {
        dao.clearAppEvents()
        dao.clearAppSessions()
        dao.clearUsageActivities()
    }

    suspend fun clearBrowserEvents() {
        dao.clearBrowserEvents()
        dao.clearBrowserActivities()
    }

    suspend fun deleteAppSession(id: Long) {
        dao.deleteAppSession(id)
    }

    suspend fun deleteBrowserEvent(id: Long) {
        dao.deleteBrowserEvent(id)
    }

    suspend fun setMinSessionSeconds(seconds: Int) {
        dao.putPreference(UserPreferenceEntity(KEY_MIN_SESSION_SECONDS, seconds.coerceAtLeast(1).toString()))
    }

    suspend fun minSessionSeconds(): Int =
        dao.getPreference(KEY_MIN_SESSION_SECONDS)?.toIntOrNull() ?: 60

    private suspend fun minSessionMs(): Long = minSessionSeconds() * 1000L

    private suspend fun mergeGapMs(): Long =
        (dao.getPreference(KEY_MERGE_GAP_SECONDS)?.toIntOrNull() ?: 15) * 1000L

    fun schedulePeriodicCollection() {
        val request = PeriodicWorkRequestBuilder<UsageCollectionWorker>(15, TimeUnit.MINUTES)
            .build()
        WorkManager.getInstance(context).enqueueUniquePeriodicWork(
            "usage-collection",
            ExistingPeriodicWorkPolicy.UPDATE,
            request
        )
    }

    fun scheduleAutoExport() {
        val request = PeriodicWorkRequestBuilder<AutoExportWorker>(6, TimeUnit.HOURS)
            .build()
        WorkManager.getInstance(context).enqueueUniquePeriodicWork(
            "auto-export",
            ExistingPeriodicWorkPolicy.UPDATE,
            request
        )
    }

    suspend fun setAutoExportEnabled(enabled: Boolean) {
        dao.putPreference(UserPreferenceEntity(KEY_AUTO_EXPORT_ENABLED, enabled.toString()))
    }

    suspend fun autoExportEnabled(): Boolean =
        dao.getPreference(KEY_AUTO_EXPORT_ENABLED)?.toBooleanStrictOrNull() ?: true

    private suspend fun findViaSessionFor(ts: Long): AppSessionEntity? =
        dao.viaSessionAt(ts)

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

    private fun BrowserEventEntity.toActivity(viaSession: AppSessionEntity?): ActivityEntity =
        ActivityEntity(
            timestamp = ts / 1000,
            appName = viaSession?.appLabel ?: "Via",
            windowTitle = title?.takeIf { it.isNotBlank() } ?: url,
            category = category,
            duration = ((durationMs ?: 0L) / 1000).coerceAtLeast(1),
            browserUrl = url,
            executablePath = browserPackage,
            semanticCategory = semanticCategory,
            semanticConfidence = semanticConfidence,
            mobileSource = "via_userscript"
        )

    private fun LocalDate.startMs(): Long =
        atStartOfDay(zone).toInstant().toEpochMilli()

    private fun LocalDate.endMs(): Long = plusDays(1).startMs()

    companion object {
        private const val KEY_LAST_COLLECTED_TS = "last_collected_ts"
        private const val KEY_LOCAL_SERVER_ENABLED = "local_server_enabled"
        private const val KEY_BROWSER_LOGGING_ENABLED = "browser_logging_enabled"
        private const val KEY_DOMAIN_ONLY = "browser_domain_only"
        private const val KEY_BLOCKED_DOMAINS = "browser_blocked_domains"
        private const val KEY_MIN_SESSION_SECONDS = "min_session_seconds"
        private const val KEY_MERGE_GAP_SECONDS = "merge_gap_seconds"
        private const val KEY_AUTO_EXPORT_ENABLED = "auto_export_enabled"
    }
}
