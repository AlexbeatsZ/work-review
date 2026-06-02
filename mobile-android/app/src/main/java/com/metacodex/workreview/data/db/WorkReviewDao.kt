package com.metacodex.workreview.data.db

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import kotlinx.coroutines.flow.Flow

data class AppUsageSummary(
    val packageName: String,
    val appLabel: String?,
    val totalDurationMs: Long
)

@Dao
interface WorkReviewDao {
    @Insert(onConflict = OnConflictStrategy.IGNORE)
    suspend fun insertAppEvents(events: List<AppEventEntity>)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertAppSessions(sessions: List<AppSessionEntity>)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertBrowserEvent(event: BrowserEventEntity)

    @Query("DELETE FROM app_sessions WHERE startTs >= :startTs AND startTs < :endTs AND source = 'usage_stats'")
    suspend fun deleteUsageSessionsBetween(startTs: Long, endTs: Long)

    @Query("SELECT * FROM app_sessions WHERE startTs >= :startTs AND startTs < :endTs ORDER BY startTs")
    fun sessionsForDay(startTs: Long, endTs: Long): Flow<List<AppSessionEntity>>

    @Query("SELECT packageName, appLabel, SUM(durationMs) AS totalDurationMs FROM app_sessions WHERE startTs >= :startTs AND startTs < :endTs GROUP BY packageName, appLabel ORDER BY totalDurationMs DESC")
    fun usageSummaryForDay(startTs: Long, endTs: Long): Flow<List<AppUsageSummary>>

    @Query("SELECT * FROM browser_events WHERE ts >= :startTs AND ts < :endTs ORDER BY ts")
    fun browserEventsForDay(startTs: Long, endTs: Long): Flow<List<BrowserEventEntity>>

    @Query("SELECT * FROM app_events ORDER BY ts DESC LIMIT :limit")
    fun recentAppEvents(limit: Int): Flow<List<AppEventEntity>>

    @Query("SELECT * FROM app_sessions ORDER BY startTs DESC LIMIT :limit")
    fun recentAppSessions(limit: Int): Flow<List<AppSessionEntity>>

    @Query("SELECT * FROM browser_events ORDER BY ts DESC LIMIT :limit")
    fun recentBrowserEvents(limit: Int): Flow<List<BrowserEventEntity>>

    @Query("SELECT * FROM app_sessions ORDER BY startTs")
    suspend fun allAppSessions(): List<AppSessionEntity>

    @Query("SELECT * FROM browser_events ORDER BY ts")
    suspend fun allBrowserEvents(): List<BrowserEventEntity>

    @Query("SELECT value FROM user_preferences WHERE `key` = :key")
    suspend fun getPreference(key: String): String?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun putPreference(preference: UserPreferenceEntity)

    @Query("DELETE FROM app_events")
    suspend fun clearAppEvents()

    @Query("DELETE FROM app_sessions")
    suspend fun clearAppSessions()

    @Query("DELETE FROM browser_events")
    suspend fun clearBrowserEvents()
}
