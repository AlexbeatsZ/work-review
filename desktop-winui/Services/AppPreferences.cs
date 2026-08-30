using System.Text.Json.Nodes;

namespace WorkReview.Services;

/// <summary>
/// UI 本地偏好（语言等），存放在数据目录下 ui-state.json，不进后端 config。
/// </summary>
public static class AppPreferences
{
    private static readonly object Gate = new();
    private static JsonObject _state = new();

    public static string DataDir { get; set; } = string.Empty;

    private static string FilePath => Path.Combine(DataDir, "ui-state.json");

    public static void Load()
    {
        try
        {
            if (File.Exists(FilePath))
            {
                _state = JsonNode.Parse(File.ReadAllText(FilePath)) as JsonObject ?? new JsonObject();
            }
        }
        catch
        {
            _state = new JsonObject();
        }
    }

    public static string? Get(string key)
    {
        lock (Gate)
        {
            return _state[key]?.GetValue<string>();
        }
    }

    public static void Set(string key, string value)
    {
        lock (Gate)
        {
            _state[key] = value;
            try
            {
                File.WriteAllText(FilePath, _state.ToJsonString(new System.Text.Json.JsonSerializerOptions
                {
                    WriteIndented = true,
                }));
            }
            catch
            {
                // 持久化失败不影响运行
            }
        }
    }
}
