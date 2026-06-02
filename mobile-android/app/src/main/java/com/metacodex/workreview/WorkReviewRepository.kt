package com.metacodex.workreview

import android.content.Context
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import com.metacodex.workreview.data.browser.PrivacyFilter
import com.metacodex.workreview.data.browser.PrivacySettings
import com.metacodex.workreview.data.db.BrowserEventEntity
import com.metacodex.workreview.data.db.UserPreferenceEntity
import com.metacodex.workreview.data.db.WorkReviewDatabase
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
        val collector = UsageStatsCollector(context)
        val result = collector.collect(start, now)
        dao.insertAppEvents(result.events)
        dao.deleteUsageSessionsBetween(start, now)
        dao.insertAppSessions(result.sessions)
        dao.putPreference(UserPreferenceEntity(KEY_LAST_COLLECTED_TS, now.toString()))
    }

    suspend fun insertBrowserEvent(event: BrowserEventEntity) {
        val settings = browserPrivacySettings()
        PrivacyFilter(settings).apply(event)?.let { dao.insertBrowserEvent(it) }
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
    }

    suspend fun clearBrowserEvents() {
        dao.clearBrowserEvents()
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

    private fun LocalDate.startMs(): Long =
        atStartOfDay(zone).toInstant().toEpochMilli()

    private fun LocalDate.endMs(): Long = plusDays(1).startMs()

    companion object {
        private const val KEY_LAST_COLLECTED_TS = "last_collected_ts"
        private const val KEY_LOCAL_SERVER_ENABLED = "local_server_enabled"
        private const val KEY_BROWSER_LOGGING_ENABLED = "browser_logging_enabled"
        private const val KEY_DOMAIN_ONLY = "browser_domain_only"
        private const val KEY_BLOCKED_DOMAINS = "browser_blocked_domains"
    }
}
