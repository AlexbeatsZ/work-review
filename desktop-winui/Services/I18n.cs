using System.Globalization;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace WorkReview.Services;

/// <summary>
/// i18n：加载 i18n/*.json（与 Svelte 版 locales 一致），dot-path 查找 + {param} 插值。
/// 语言持久化到数据目录旁的 ui-state.json。
/// </summary>
public static class I18n
{
    public static readonly string[] SupportedLocales = { "zh-CN", "en", "zh-TW" };
    private static readonly string[] Cycle = { "zh-CN", "en", "zh-TW" };

    private static readonly Dictionary<string, (string Short, string Label)> LocaleMeta = new()
    {
        ["zh-CN"] = ("ZH", "简体中文"),
        ["en"] = ("EN", "English"),
        ["zh-TW"] = ("TW", "繁體中文"),
    };

    private static readonly Dictionary<string, string> CategoryLabels = new()
    {
        ["development|zh-CN"] = "开发工具", ["development|en"] = "Development", ["development|zh-TW"] = "開發工具",
        ["browser|zh-CN"] = "浏览器", ["browser|en"] = "Browser", ["browser|zh-TW"] = "瀏覽器",
        ["communication|zh-CN"] = "通讯协作", ["communication|en"] = "Communication", ["communication|zh-TW"] = "通訊協作",
        ["office|zh-CN"] = "办公软件", ["office|en"] = "Office", ["office|zh-TW"] = "辦公軟體",
        ["design|zh-CN"] = "设计工具", ["design|en"] = "Design", ["design|zh-TW"] = "設計工具",
        ["entertainment|zh-CN"] = "娱乐摸鱼", ["entertainment|en"] = "Leisure", ["entertainment|zh-TW"] = "娛樂摸魚",
        ["other|zh-CN"] = "其他", ["other|en"] = "Other", ["other|zh-TW"] = "其他",
    };

    private static readonly (string Key, string Zh, string En, string Tw)[] SemanticLabels =
    {
        ("编码开发", "编码开发", "Development", "編碼開發"),
        ("内容撰写", "内容撰写", "Writing", "內容撰寫"),
        ("资料阅读", "资料阅读", "Reading", "資料閱讀"),
        ("资料调研", "资料调研", "Research", "資料調研"),
        ("任务规划", "任务规划", "Planning", "任務規劃"),
        ("设计创作", "设计创作", "Design", "設計創作"),
        ("AI 协作", "AI 协作", "AI Collaboration", "AI 協作"),
        ("即时聊天", "即时聊天", "Chat", "即時聊天"),
        ("会议沟通", "会议沟通", "Meetings", "會議溝通"),
        ("视频内容", "视频内容", "Video", "影片內容"),
        ("音乐音频", "音乐音频", "Audio", "音樂音訊"),
        ("休息娱乐", "休息娱乐", "Leisure", "休息娛樂"),
        ("未知活动", "未知活动", "Unknown", "未知活動"),
    };

    private const string DefaultLocale = "zh-CN";

    private static Dictionary<string, JsonNode> _messages = new();
    private static string _locale = DefaultLocale;

    /// <summary>当前语言变化时触发（UI 线程）。</summary>
    public static event Action<string>? LocaleChanged;

    public static string Locale => _locale;

    public static string ShortLabel => LocaleMeta.TryGetValue(_locale, out var m) ? m.Short : "ZH";

    public static void Initialize(string? preferred = null)
    {
        foreach (var loc in SupportedLocales)
        {
            var path = Path.Combine(AppContext.BaseDirectory, "i18n", $"{loc}.json");
            if (!File.Exists(path))
            {
                continue;
            }
            try
            {
                _messages[loc] = JsonNode.Parse(File.ReadAllText(path)) ?? new JsonObject();
            }
            catch
            {
                // 单个语言文件损坏不影响其他语言
            }
        }

        _locale = Normalize(preferred) ?? NormalizeWin32() ?? DefaultLocale;
    }

    private static string? Normalize(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return null;
        }
        var v = value.Trim();
        if (SupportedLocales.Contains(v))
        {
            return v;
        }
        var lower = v.ToLowerInvariant();
        if (lower.StartsWith("zh-tw") || lower.StartsWith("zh-hk"))
        {
            return "zh-TW";
        }
        if (lower.StartsWith("zh"))
        {
            return "zh-CN";
        }
        if (lower.StartsWith("en"))
        {
            return "en";
        }
        return null;
    }

    private static string? NormalizeWin32()
    {
        try
        {
            return Normalize(CultureInfo.CurrentUICulture.Name);
        }
        catch
        {
            return null;
        }
    }

    public static void SetLocale(string locale)
    {
        var next = Normalize(locale) ?? DefaultLocale;
        if (next == _locale)
        {
            return;
        }
        _locale = next;
        LocaleChanged?.Invoke(next);
    }

    public static void CycleLocale()
    {
        var idx = Array.IndexOf(Cycle, _locale);
        SetLocale(Cycle[(idx + 1 + Cycle.Length) % Cycle.Length]);
    }

    private static JsonNode? Resolve(string key)
    {
        if (!_messages.TryGetValue(_locale, out var root) ||
            ResolvePath(root, key) is not { } value)
        {
            _messages.TryGetValue(DefaultLocale, out var fallback);
            value = fallback is null ? null : ResolvePath(fallback, key);
        }
        return value;
    }

    private static JsonNode? ResolvePath(JsonNode? node, string key)
    {
        var current = node;
        foreach (var segment in key.Split('.'))
        {
            current = current?[segment];
            if (current is null)
            {
                return null;
            }
        }
        return current;
    }

    public static string T(string key, params (string Name, object? Value)[] parameters)
    {
        var raw = Resolve(key)?.GetValue<string>() ?? key;
        foreach (var (name, value) in parameters)
        {
            raw = raw.Replace($"{{{name}}}", value?.ToString() ?? string.Empty);
        }
        return raw;
    }

    public static string T(string key, Dictionary<string, object?> parameters)
    {
        var raw = Resolve(key)?.GetValue<string>() ?? key;
        foreach (var (name, value) in parameters)
        {
            raw = raw.Replace($"{{{name}}}", value?.ToString() ?? string.Empty);
        }
        return raw;
    }

    public static string? Tm(string key) => Resolve(key)?.GetValue<string>();

    public static string TranslateCategoryLabel(string key) =>
        CategoryLabels.TryGetValue($"{key}|{_locale}", out var label)
            ? label
            : CategoryLabels.TryGetValue($"{key}|{DefaultLocale}", out var fallback)
                ? fallback
                : key;

    public static string TranslateSemanticCategoryLabel(string label)
    {
        foreach (var (key, zh, en, tw) in SemanticLabels)
        {
            if (key == label)
            {
                return _locale switch
                {
                    "en" => en,
                    "zh-TW" => tw,
                    _ => zh,
                };
            }
        }
        return label;
    }

    /// <summary>时长本地化（与 formatDurationLocalized 行为一致）。</summary>
    public static string FormatDuration(long seconds, bool compact = false)
    {
        var isTw = _locale == "zh-TW";
        var hourUnit = isTw ? (compact ? "時" : "小時") : (compact ? "h" : "小时");
        var minuteUnit = isTw ? (compact ? "分" : "分鐘") : (compact ? "m" : "分钟");
        var secondUnit = "秒";
        var isEn = _locale == "en";

        if (seconds <= 0)
        {
            if (isEn)
            {
                return compact ? "0m" : "0s";
            }
            return $"0{minuteUnit}";
        }

        var hours = seconds / 3600;
        var minutes = seconds % 3600 / 60;
        var secs = seconds % 60;

        if (isEn)
        {
            if (hours > 0)
            {
                return compact
                    ? (minutes > 0 ? $"{hours}h{minutes}m" : $"{hours}h")
                    : (minutes > 0 ? $"{hours}h {minutes}m" : $"{hours}h");
            }
            if (minutes > 0)
            {
                return $"{minutes}m";
            }
            return $"{secs}s";
        }

        if (hours > 0)
        {
            return minutes > 0 ? $"{hours}{hourUnit}{minutes}{minuteUnit}" : $"{hours}{hourUnit}";
        }
        if (minutes > 0)
        {
            return $"{minutes}{minuteUnit}";
        }
        return $"{secs}{secondUnit}";
    }

    public static string FormatTime(long timestampSeconds, string format) =>
        DateTimeOffset.FromUnixTimeSeconds(timestampSeconds).ToLocalTime().DateTime.ToString(format);
}
