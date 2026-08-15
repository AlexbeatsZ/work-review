package com.metacodex.workreview

import android.app.Application
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

class WorkReviewApp : Application() {
    lateinit var repository: WorkReviewRepository
        private set
    private val appScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    override fun onCreate() {
        super.onCreate()
        repository = WorkReviewRepository(this)
        repository.schedulePeriodicCollection()
        appScope.launch {
            if (repository.autoExportEnabled()) {
                repository.scheduleTimedAutoExports()
            }
        }
    }
}
