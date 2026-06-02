package com.metacodex.workreview.data.db

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase

@Database(
    entities = [
        AppEventEntity::class,
        AppSessionEntity::class,
        BrowserEventEntity::class,
        UserPreferenceEntity::class
    ],
    version = 1,
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
                ).build().also { instance = it }
            }
    }
}
