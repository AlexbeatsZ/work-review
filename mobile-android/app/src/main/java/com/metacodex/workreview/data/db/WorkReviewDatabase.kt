package com.metacodex.workreview.data.db

import android.content.Context
import androidx.room.Database
import androidx.room.migration.Migration
import androidx.room.Room
import androidx.room.RoomDatabase
import androidx.sqlite.db.SupportSQLiteDatabase

@Database(
    entities = [
        AppEventEntity::class,
        AppSessionEntity::class,
        BrowserEventEntity::class,
        ActivityEntity::class,
        UserPreferenceEntity::class
    ],
    version = 2,
    exportSchema = true
)
abstract class WorkReviewDatabase : RoomDatabase() {
    abstract fun dao(): WorkReviewDao

    companion object {
        @Volatile private var instance: WorkReviewDatabase? = null

        fun get(context: Context): WorkReviewDatabase =
            instance ?: synchronized(this) {
                instance ?: Room.databaseBuilder(
                    context.applicationContext,
                    WorkReviewDatabase::class.java,
                    "work_review_mobile.db"
                )
                    .addMigrations(MIGRATION_1_2)
                    .build()
                    .also { instance = it }
            }

        private val MIGRATION_1_2 = object : Migration(1, 2) {
            override fun migrate(db: SupportSQLiteDatabase) {
                db.execSQL("ALTER TABLE app_sessions ADD COLUMN category TEXT NOT NULL DEFAULT 'other'")
                db.execSQL("ALTER TABLE app_sessions ADD COLUMN semanticCategory TEXT")
                db.execSQL("ALTER TABLE app_sessions ADD COLUMN semanticConfidence INTEGER")
                db.execSQL("ALTER TABLE browser_events ADD COLUMN viaSessionId INTEGER")
                db.execSQL("ALTER TABLE browser_events ADD COLUMN category TEXT NOT NULL DEFAULT 'browser'")
                db.execSQL("ALTER TABLE browser_events ADD COLUMN semanticCategory TEXT")
                db.execSQL("ALTER TABLE browser_events ADD COLUMN semanticConfidence INTEGER")
                db.execSQL(
                    """
                    CREATE TABLE IF NOT EXISTS activities (
                        id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                        timestamp INTEGER NOT NULL,
                        app_name TEXT NOT NULL,
                        window_title TEXT NOT NULL,
                        screenshot_path TEXT NOT NULL DEFAULT '',
                        ocr_text TEXT,
                        category TEXT NOT NULL,
                        duration INTEGER NOT NULL,
                        browser_url TEXT,
                        executable_path TEXT,
                        semantic_category TEXT,
                        semantic_confidence INTEGER,
                        intent_purpose TEXT,
                        intent_note TEXT,
                        intent_start_timestamp INTEGER,
                        intent_end_timestamp INTEGER,
                        intent_completed_at INTEGER,
                        mobile_source TEXT NOT NULL DEFAULT 'android'
                    )
                    """.trimIndent()
                )
                db.execSQL("CREATE INDEX IF NOT EXISTS index_activities_timestamp_app_name ON activities (timestamp, app_name)")
                db.execSQL("CREATE INDEX IF NOT EXISTS index_activities_browser_url ON activities (browser_url)")
            }
        }
    }
}
