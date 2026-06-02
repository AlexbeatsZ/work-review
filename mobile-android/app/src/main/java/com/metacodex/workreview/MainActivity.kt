package com.metacodex.workreview

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Divider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.metacodex.workreview.data.ExportWriter
import com.metacodex.workreview.data.db.AppEventEntity
import com.metacodex.workreview.data.db.AppSessionEntity
import com.metacodex.workreview.data.db.BrowserEventEntity
import com.metacodex.workreview.permissions.UsageAccess
import com.metacodex.workreview.server.BrowserLogServer
import kotlinx.coroutines.launch
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter

class MainActivity : ComponentActivity() {
    private var server: BrowserLogServer? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val repository = (application as WorkReviewApp).repository
        setContent {
            WorkReviewMobileApp(
                repository = repository,
                isServerRunning = { server != null },
                startServer = {
                    if (server == null) {
                        server = BrowserLogServer(repository).also { it.start() }
                    }
                },
                stopServer = {
                    server?.stop()
                    server = null
                }
            )
        }
    }

    override fun onDestroy() {
        server?.stop()
        server = null
        super.onDestroy()
    }
}

@Composable
private fun WorkReviewMobileApp(
    repository: WorkReviewRepository,
    isServerRunning: () -> Boolean,
    startServer: () -> Unit,
    stopServer: () -> Unit
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var selectedDate by remember { mutableStateOf(LocalDate.now()) }
    var tab by remember { mutableStateOf("overview") }
    var hasUsageAccess by remember { mutableStateOf(UsageAccess.hasUsageAccess(context)) }
    var serverRunning by remember { mutableStateOf(isServerRunning()) }
    var status by remember { mutableStateOf("") }

    LaunchedEffect(Unit) {
        repository.collectUsageNow()
        hasUsageAccess = UsageAccess.hasUsageAccess(context)
    }

    MaterialTheme {
        Surface(modifier = Modifier.fillMaxSize()) {
            Scaffold(
                bottomBar = {
                    NavigationBar {
                        listOf("overview" to "总览", "timeline" to "时间线", "settings" to "设置", "debug" to "调试").forEach { (id, label) ->
                            NavigationBarItem(
                                selected = tab == id,
                                onClick = { tab = id },
                                label = { Text(label) },
                                icon = { Text(label.first().toString()) }
                            )
                        }
                    }
                }
            ) { padding ->
                Column(
                    modifier = Modifier
                        .padding(padding)
                        .padding(16.dp)
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    Header(
                        selectedDate = selectedDate,
                        onPrev = { selectedDate = selectedDate.minusDays(1) },
                        onToday = { selectedDate = LocalDate.now() },
                        onNext = { selectedDate = selectedDate.plusDays(1) },
                        onCollect = {
                            scope.launch {
                                repository.collectUsageNow()
                                hasUsageAccess = UsageAccess.hasUsageAccess(context)
                                status = "已补采到当前时间"
                            }
                        }
                    )
                    if (!hasUsageAccess) {
                        PermissionCard { context.startActivity(UsageAccess.settingsIntent()) }
                    }
                    if (status.isNotBlank()) Text(status, color = MaterialTheme.colorScheme.primary)
                    when (tab) {
                        "overview" -> OverviewScreen(repository, selectedDate)
                        "timeline" -> TimelineScreen(repository, selectedDate)
                        "settings" -> SettingsScreen(
                            repository = repository,
                            serverRunning = serverRunning,
                            onToggleServer = {
                                if (serverRunning) {
                                    stopServer()
                                    serverRunning = false
                                } else {
                                    startServer()
                                    serverRunning = true
                                }
                                scope.launch { repository.setLocalServerEnabled(serverRunning) }
                            },
                            onStatus = { status = it }
                        )
                        "debug" -> DebugScreen(repository, hasUsageAccess, serverRunning)
                    }
                }
            }
        }
    }
}

@Composable
private fun Header(selectedDate: LocalDate, onPrev: () -> Unit, onToday: () -> Unit, onNext: () -> Unit, onCollect: () -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("Work Review Mobile", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            TextButton(onClick = onPrev) { Text("前一天") }
            TextButton(onClick = onToday) { Text(selectedDate.toString()) }
            TextButton(onClick = onNext) { Text("后一天") }
        }
        Button(onClick = onCollect, modifier = Modifier.fillMaxWidth()) { Text("立即补采使用记录") }
    }
}

@Composable
private fun PermissionCard(onOpenSettings: () -> Unit) {
    Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer)) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("需要开启使用情况访问权限", fontWeight = FontWeight.Bold)
            Text("授权后才能通过 UsageStatsManager 读取 app 使用事件。")
            Button(onClick = onOpenSettings) { Text("打开系统 Usage Access 设置") }
        }
    }
}

@Composable
private fun OverviewScreen(repository: WorkReviewRepository, date: LocalDate) {
    val summary by repository.usageSummary(date).collectAsState(initial = emptyList())
    val browserEvents by repository.browserEvents(date).collectAsState(initial = emptyList())
    Section("今日 app 使用排行") {
        if (summary.isEmpty()) EmptyText()
        summary.forEachIndexed { index, row ->
            Text("${index + 1}. ${row.appLabel ?: row.packageName}  ${formatDuration(row.totalDurationMs)}")
        }
    }
    Section("Via URL 记录") {
        val viaEvents = browserEvents.takeLast(12)
        if (viaEvents.isEmpty()) EmptyText()
        viaEvents.forEach { Text("${formatTime(it.ts)} ${it.eventType} ${it.title ?: it.url}") }
    }
}

@Composable
private fun TimelineScreen(repository: WorkReviewRepository, date: LocalDate) {
    val sessions by repository.sessions(date).collectAsState(initial = emptyList())
    val browserEvents by repository.browserEvents(date).collectAsState(initial = emptyList())
    Section("App 时间线") {
        if (sessions.isEmpty()) EmptyText()
        sessions.forEach { SessionRow(it) }
    }
    Section("URL 时间线") {
        if (browserEvents.isEmpty()) EmptyText()
        browserEvents.forEach { BrowserEventRow(it) }
    }
}

@Composable
private fun SettingsScreen(
    repository: WorkReviewRepository,
    serverRunning: Boolean,
    onToggleServer: () -> Unit,
    onStatus: (String) -> Unit
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    Section("本地接收服务") {
        Text("状态：${if (serverRunning) "运行中" else "已关闭"}")
        Button(onClick = onToggleServer) {
            Text(if (serverRunning) "关闭 127.0.0.1:17890/log" else "启动 127.0.0.1:17890/log")
        }
    }
    Section("隐私与清理") {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = { scope.launch { repository.clearAppUsage(); onStatus("已清空 app 使用记录") } }) { Text("清空 app 记录") }
            Button(onClick = { scope.launch { repository.clearBrowserEvents(); onStatus("已清空浏览记录") } }) { Text("清空浏览记录") }
        }
    }
    Section("导出") {
        Button(onClick = {
            scope.launch {
                val dir = ExportWriter(context).exportAll()
                onStatus("已导出到 ${dir.absolutePath}")
            }
        }) {
            Text("导出 SQLite 和 CSV")
        }
    }
}

@Composable
private fun DebugScreen(repository: WorkReviewRepository, hasUsageAccess: Boolean, serverRunning: Boolean) {
    val appEvents by repository.recentAppEvents().collectAsState(initial = emptyList())
    val appSessions by repository.recentAppSessions().collectAsState(initial = emptyList())
    val browserEvents by repository.recentBrowserEvents().collectAsState(initial = emptyList())
    Section("状态") {
        Text("Usage Access：${if (hasUsageAccess) "已开启" else "未开启"}")
        Text("本地 HTTP 服务：${if (serverRunning) "运行中" else "已关闭"}")
        Text("Via 包名：mark.via.gp")
    }
    Section("最近 app_events") { appEvents.forEach { AppEventRow(it) } }
    Section("最近 app_sessions") { appSessions.forEach { SessionRow(it) } }
    Section("最近 browser_events") { browserEvents.forEach { BrowserEventRow(it) } }
}

@Composable
private fun Section(title: String, content: @Composable ColumnScope.() -> Unit) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(title, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
            Divider()
            content()
        }
    }
}

@Composable private fun EmptyText() = Text("暂无数据")

@Composable
private fun SessionRow(session: AppSessionEntity) {
    Text("${formatTime(session.startTs)}-${formatTime(session.endTs)} ${session.appLabel ?: session.packageName} ${formatDuration(session.durationMs)}")
}

@Composable
private fun BrowserEventRow(event: BrowserEventEntity) {
    Text("${formatTime(event.ts)} ${event.eventType} ${event.title ?: event.url}")
}

@Composable
private fun AppEventRow(event: AppEventEntity) {
    Text("${formatTime(event.ts)} ${event.packageName} type=${event.eventType}")
}

private fun formatDuration(ms: Long): String {
    val minutes = ms / 60_000
    val hours = minutes / 60
    val rem = minutes % 60
    return if (hours > 0) "${hours}h ${rem}m" else "${rem}m"
}

private fun formatTime(ms: Long): String =
    DateTimeFormatter.ofPattern("HH:mm")
        .format(Instant.ofEpochMilli(ms).atZone(ZoneId.systemDefault()))
