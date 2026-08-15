package com.metacodex.workreview

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.metacodex.workreview.ui.WorkReviewMobileApp
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    private lateinit var repository: WorkReviewRepository

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        repository = (application as WorkReviewApp).repository
        setContent { WorkReviewMobileApp(repository) }
    }

    override fun onResume() {
        super.onResume()
        if (::repository.isInitialized) {
            MainScope().launch { repository.collectUsageNow() }
        }
    }
}
