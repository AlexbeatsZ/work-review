using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace WorkReview.Engine;

/// <summary>引擎返回 ok=false 时抛出，message 为后端错误文本。</summary>
public sealed class EngineException : Exception
{
    public EngineException(string message) : base(message) { }
}

/// <summary>
/// 引擎门面：阻塞调用走线程池，事件经原生回调转发到托管事件。
/// 后端方法名与参数键与 Tauri 命令一一对应（snake_case）。
/// </summary>
public static class EngineApi
{
    private static EngineNative.EngineEventCallback? _pinnedCallback;

    /// <summary>引擎事件（screenshot-taken / recording-state-changed / config-changed）。
    /// 触发线程为 Rust 工作线程，订阅方必须自行调度回 UI 线程。</summary>
    public static event Action<string, JsonNode>? EngineEvent;

    public static JsonSerializerOptions JsonOptions { get; } = new()
    {
        PropertyNameCaseInsensitive = true,
        NumberHandling = System.Text.Json.Serialization.JsonNumberHandling.AllowReadingFromString,
    };

    public static void Initialize()
    {
        _pinnedCallback = OnNativeEvent;
        EngineNative.engine_set_event_callback(_pinnedCallback);

        var code = EngineNative.engine_start();
        if (code != EngineNative.StartOk)
        {
            throw new InvalidOperationException($"work_review_engine 启动失败（code={code}）");
        }
    }

    private static void OnNativeEvent(IntPtr eventName, IntPtr payload)
    {
        try
        {
            var name = Marshal.PtrToStringUTF8(eventName) ?? string.Empty;
            var payloadText = Marshal.PtrToStringUTF8(payload) ?? "null";
            var node = JsonNode.Parse(payloadText) ?? new JsonObject();
            EngineEvent?.Invoke(name, node);
        }
        catch
        {
            // 事件反序列化失败不应影响引擎线程
        }
    }

    /// <summary>在调用方线程上阻塞执行（请在 Task.Run 中调用）。</summary>
    public static JsonNode? Invoke(string method, object? args = null)
    {
        var argsJson = JsonSerializer.Serialize(args ?? new { });
        var ptr = EngineNative.engine_invoke(method, argsJson);
        try
        {
            var text = Marshal.PtrToStringUTF8(ptr) ?? "{\"ok\":false,\"error\":\"空响应\"}";
            var envelope = JsonNode.Parse(text) ?? throw new EngineException("无法解析引擎响应");
            var ok = envelope["ok"]?.GetValue<bool>() ?? false;
            if (!ok)
            {
                var error = envelope["error"]?.GetValue<string>() ?? "未知引擎错误";
                throw new EngineException(error);
            }
            return envelope["value"];
        }
        finally
        {
            EngineNative.engine_free_string(ptr);
        }
    }

    /// <summary>线程池上执行并返回强类型结果。</summary>
    public static Task<T> InvokeAsync<T>(string method, object? args = null) =>
        Task.Run(() =>
        {
            var node = Invoke(method, args);
            return node.Deserialize<T>(JsonOptions)
                   ?? throw new EngineException($"无法将 {method} 响应映射为 {typeof(T).Name}");
        });

    /// <summary>线程池上执行，忽略返回值。</summary>
    public static Task InvokeAsync(string method, object? args = null) =>
        Task.Run(() => Invoke(method, args));

    public static Task<JsonNode?> InvokeNodeAsync(string method, object? args = null) =>
        Task.Run(() => Invoke(method, args));
}
