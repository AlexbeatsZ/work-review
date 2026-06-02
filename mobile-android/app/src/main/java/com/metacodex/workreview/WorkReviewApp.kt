package com.metacodex.workreview

import android.app.Application

class WorkReviewApp : Application() {
    lateinit var repository: WorkReviewRepository
        private set

    override fun onCreate() {
        super.onCreate()
        repository = WorkReviewRepository(this)
        repository.schedulePeriodicCollection()
    }
}
