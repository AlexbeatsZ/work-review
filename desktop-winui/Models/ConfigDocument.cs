using System.Text.Json.Nodes;
using WorkReview.Engine;

namespace WorkReview.Models;

/// <summary>
/// 配置文档：保留完整 config.json 的 JsonNode（save_config 原样回传，避免字段丢失），
/// 同时为设置页提供类型化读写。
/// </summary>
public sealed class ConfigDocument
{
    private readonly JsonObject _root;

    private ConfigDocument(JsonObject root)
    {
        _root = root;
    }

    public static async Task<ConfigDocument> LoadAsync()
    {
        var node = await EngineApi.InvokeNodeAsync("get_config");
        return new ConfigDocument(node as JsonObject ?? new JsonObject());
    }

    public JsonObject Root => _root;

    public Task SaveAsync() => EngineApi.InvokeAsync("save_config", new { config = _root });

    public ConfigDocument Clone()
    {
        var copy = JsonNode.Parse(_root.ToJsonString()) as JsonObject ?? new JsonObject();
        return new ConfigDocument(copy);
    }

    // ---------- 通用访问器 ----------

    private JsonObject? Obj(string path)
    {
        var current = (JsonNode?)_root;
        foreach (var segment in path.Split('.'))
        {
            current = current?[segment];
        }
        return current as JsonObject;
    }

    private JsonArray? Arr(string path)
    {
        var current = (JsonNode?)_root;
        foreach (var segment in path.Split('.'))
        {
            current = current?[segment];
        }
        return current as JsonArray;
    }

    public T Get<T>(string path, T fallback)
    {
        var current = (JsonNode?)_root;
        foreach (var segment in path.Split('.'))
        {
            current = current?[segment];
        }
        if (current is null)
        {
            return fallback;
        }
        try
        {
            return current.GetValue<T>();
        }
        catch
        {
            return fallback;
        }
    }

    public void Set(string path, JsonNode? value)
    {
        var segments = path.Split('.');
        var current = (JsonNode)_root;
        for (var i = 0; i < segments.Length - 1; i++)
        {
            var next = current[segments[i]];
            if (next is not JsonObject)
            {
                next = new JsonObject();
                current[segments[i]] = next;
            }
            current = next;
        }
        current[segments[^1]] = value;
    }

    public void Set(string path, long value) => Set(path, JsonValue.Create(value));
    public void Set(string path, double value) => Set(path, JsonValue.Create(value));
    public void Set(string path, bool value) => Set(path, JsonValue.Create(value));
    public void Set(string path, string? value) => Set(path, value is null ? null : JsonValue.Create(value));

    // ---------- 常用字段（设置页使用） ----------

    public long ScreenshotInterval
    {
        get => Get("screenshot_interval", 30L);
        set => Set("screenshot_interval", value);
    }

    public long IdleThresholdMinutes
    {
        get => Get("idle_threshold_minutes", 5L);
        set => Set("idle_threshold_minutes", value);
    }

    public bool AutoStart
    {
        get => Get("auto_start", false);
        set => Set("auto_start", value);
    }

    public bool AutoStartSilent
    {
        get => Get("auto_start_silent", true);
        set => Set("auto_start_silent", value);
    }

    public bool WorkTimeEnabled
    {
        get => Get("work_time_enabled", true);
        set => Set("work_time_enabled", value);
    }

    public long WorkStartHour
    {
        get => Get("work_start_hour", 0L);
        set => Set("work_start_hour", value);
    }

    public long WorkStartMinute
    {
        get => Get("work_start_minute", 0L);
        set => Set("work_start_minute", value);
    }

    public long WorkEndHour
    {
        get => Get("work_end_hour", 0L);
        set => Set("work_end_hour", value);
    }

    public long WorkEndMinute
    {
        get => Get("work_end_minute", 0L);
        set => Set("work_end_minute", value);
    }

    public bool LightweightMode
    {
        get => Get("lightweight_mode", true);
        set => Set("lightweight_mode", value);
    }

    public bool HideDecorations
    {
        get => Get("hide_decorations", false);
        set => Set("hide_decorations", value);
    }

    public string? DatabasePath
    {
        get => Get<string?>("database_path", null);
        set => Set("database_path", value);
    }

    public string Theme
    {
        get => Get("theme", "dark") ?? "dark";
        set => Set("theme", value);
    }

    public long ScreenshotRetentionDays
    {
        get => Get("storage.screenshot_retention_days", 7L);
        set => Set("storage.screenshot_retention_days", value);
    }

    public long MetadataRetentionDays
    {
        get => Get("storage.metadata_retention_days", 30L);
        set => Set("storage.metadata_retention_days", value);
    }

    public long StorageLimitMb
    {
        get => Get("storage.storage_limit_mb", 2048L);
        set => Set("storage.storage_limit_mb", value);
    }

    public long JpegQuality
    {
        get => Get("storage.jpeg_quality", 85L);
        set => Set("storage.jpeg_quality", value);
    }

    public long MaxImageWidth
    {
        get => Get("storage.max_image_width", 1280L);
        set => Set("storage.max_image_width", value);
    }

    public bool ScreenshotsEnabled
    {
        get => Get("storage.screenshots_enabled", true);
        set => Set("storage.screenshots_enabled", value);
    }

    public string ScreenshotDisplayMode
    {
        get => Get("storage.screenshot_display_mode", "active_window") ?? "active_window";
        set => Set("storage.screenshot_display_mode", value);
    }

    public string ScreenshotWidthMode
    {
        get => Get("storage.screenshot_width_mode", "auto") ?? "auto";
        set => Set("storage.screenshot_width_mode", value);
    }

    public JsonArray? AppPrivacyRules => Arr("privacy.app_rules");
    public JsonArray ExcludedKeywords => EnsureArray("privacy.excluded_keywords");
    public JsonArray ExcludedDomains => EnsureArray("privacy.excluded_domains");
    public JsonArray AppCategoryRules => EnsureArray("app_category_rules");
    public JsonArray CustomCategories => EnsureArray("custom_categories");
    public JsonArray WebsiteSemanticRules => EnsureArray("website_semantic_rules");
    public JsonArray CustomSemanticCategories => EnsureArray("custom_semantic_categories");

    private JsonArray EnsureArray(string path)
    {
        var arr = Arr(path);
        if (arr is null)
        {
            arr = new JsonArray();
            Set(path, arr);
        }
        return arr;
    }
}
