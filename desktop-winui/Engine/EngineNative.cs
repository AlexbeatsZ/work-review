using System.Runtime.InteropServices;

namespace WorkReview.Engine;

/// <summary>work_review_engine.dll 原生导出的 P/Invoke 绑定。</summary>
internal static class EngineNative
{
    private const string DllName = "work_review_engine";

    public const int StartOk = 0;

    [UnmanagedFunctionPointer(CallingConvention.Winapi)]
    public delegate void EngineEventCallback(IntPtr eventName, IntPtr payload);

    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "engine_set_event_callback")]
    public static extern void engine_set_event_callback(
        [MarshalAs(UnmanagedType.FunctionPtr)] EngineEventCallback callback);

    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "engine_start")]
    public static extern int engine_start();

    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "engine_invoke")]
    public static extern IntPtr engine_invoke(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string method,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string argsJson);

    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "engine_free_string")]
    public static extern void engine_free_string(IntPtr ptr);
}
