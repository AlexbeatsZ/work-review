using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

namespace WorkReview.Models;

/// <summary>活动记录（对应后端 database::Activity，snake_case JSON）。</summary>
public sealed class Activity
{
    [JsonPropertyName("id")] public long? Id { get; set; }
    [JsonPropertyName("timestamp")] public long Timestamp { get; set; }
    [JsonPropertyName("app_name")] public string AppName { get; set; } = string.Empty;
    [JsonPropertyName("window_title")] public string WindowTitle { get; set; } = string.Empty;
    [JsonPropertyName("screenshot_path")] public string ScreenshotPath { get; set; } = string.Empty;
    [JsonPropertyName("ocr_text")] public string? OcrText { get; set; }
    [JsonPropertyName("category")] public string Category { get; set; } = "other";
    [JsonPropertyName("duration")] public long Duration { get; set; }
    [JsonPropertyName("browser_url")] public string? BrowserUrl { get; set; }
    [JsonPropertyName("executable_path")] public string? ExecutablePath { get; set; }
    [JsonPropertyName("semantic_category")] public string? SemanticCategory { get; set; }
    [JsonPropertyName("semantic_confidence")] public int? SemanticConfidence { get; set; }

    public bool EqualsApp(Activity other) =>
        string.Equals(AppName?.Trim(), other.AppName?.Trim(), StringComparison.OrdinalIgnoreCase);
}

public sealed class CategoryInfo
{
    [JsonPropertyName("key")] public string Key { get; set; } = "other";
    [JsonPropertyName("name")] public string Name { get; set; } = string.Empty;
    [JsonPropertyName("color")] public string Color { get; set; } = "#64748B";
    [JsonPropertyName("icon")] public string Icon { get; set; } = "📁";
    [JsonPropertyName("is_custom")] public bool IsCustom { get; set; }
}

public sealed class SemanticCategoryInfo
{
    [JsonPropertyName("key")] public string Key { get; set; } = string.Empty;
    [JsonPropertyName("name")] public string Name { get; set; } = string.Empty;
    [JsonPropertyName("is_custom")] public bool IsCustom { get; set; }
}

/// <summary>小时摘要（对应 database::HourlySummary）。</summary>
public sealed class HourlySummary
{
    [JsonPropertyName("id")] public long? Id { get; set; }
    [JsonPropertyName("date")] public string Date { get; set; } = string.Empty;
    [JsonPropertyName("hour")] public int Hour { get; set; }
    [JsonPropertyName("summary")] public string Summary { get; set; } = string.Empty;
    [JsonPropertyName("main_apps")] public string MainApps { get; set; } = string.Empty;
    [JsonPropertyName("activity_count")] public int ActivityCount { get; set; }
    [JsonPropertyName("total_duration")] public long TotalDuration { get; set; }
    [JsonPropertyName("representative_screenshots")] public string? RepresentativeScreenshots { get; set; }
    [JsonPropertyName("created_at")] public long CreatedAt { get; set; }
}

/// <summary>意图识别 / 工作段结果（work_intelligence::IntentAnalysisResult）。</summary>
public sealed class IntentAnalysisResult
{
    [JsonPropertyName("sessions")] public List<WorkSession> Sessions { get; set; } = new();
    [JsonPropertyName("summary")] public List<IntentSummary> Summary { get; set; } = new();
}

public sealed class IntentSummary
{
    [JsonPropertyName("label")] public string Label { get; set; } = string.Empty;
    [JsonPropertyName("duration")] public long Duration { get; set; }
    [JsonPropertyName("session_count")] public int SessionCount { get; set; }
}

public sealed class NamedDuration
{
    [JsonPropertyName("name")] public string Name { get; set; } = string.Empty;
    [JsonPropertyName("duration")] public long Duration { get; set; }
}

/// <summary>工作段（WorkSession，camelCase JSON）。</summary>
public sealed class WorkSession
{
    [JsonPropertyName("sessionId")] public string SessionId { get; set; } = string.Empty;
    [JsonPropertyName("date")] public string Date { get; set; } = string.Empty;
    [JsonPropertyName("startTimestamp")] public long StartTimestamp { get; set; }
    [JsonPropertyName("endTimestamp")] public long EndTimestamp { get; set; }
    [JsonPropertyName("duration")] public long Duration { get; set; }
    [JsonPropertyName("activityCount")] public int ActivityCount { get; set; }
    [JsonPropertyName("appCount")] public int AppCount { get; set; }
    [JsonPropertyName("dominantApp")] public string DominantApp { get; set; } = string.Empty;
    [JsonPropertyName("dominantCategory")] public string DominantCategory { get; set; } = string.Empty;
    [JsonPropertyName("title")] public string Title { get; set; } = string.Empty;
    [JsonPropertyName("browserDomains")] public List<string> BrowserDomains { get; set; } = new();
    [JsonPropertyName("topApps")] public List<NamedDuration> TopApps { get; set; } = new();
    [JsonPropertyName("topKeywords")] public List<string> TopKeywords { get; set; } = new();
    [JsonPropertyName("intentLabel")] public string IntentLabel { get; set; } = string.Empty;
    [JsonPropertyName("intentConfidence")] public int IntentConfidence { get; set; }
    [JsonPropertyName("intentEvidence")] public List<string> IntentEvidence { get; set; } = new();
}

/// <summary>recording-state-changed 事件负载（camelCase）。</summary>
public sealed class RecordingStatePayload
{
    [JsonPropertyName("isRecording")] public bool IsRecording { get; set; }
    [JsonPropertyName("isPaused")] public bool IsPaused { get; set; }
}

/// <summary>存储统计（get_storage_stats 实际返回）。</summary>
public sealed class StorageStats
{
    [JsonPropertyName("total_files")] public long TotalFiles { get; set; }
    [JsonPropertyName("total_size_mb")] public string TotalSizeMb { get; set; } = "0.0";
    [JsonPropertyName("storage_limit_mb")] public int StorageLimitMb { get; set; }
    [JsonPropertyName("retention_days")] public int RetentionDays { get; set; }
}
