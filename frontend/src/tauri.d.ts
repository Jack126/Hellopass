// Tauri 全局 API 类型声明（tauri.conf.json 已开启 withGlobalTauri）。
// 这样无需安装 @tauri-apps/api 即可直接调用 window.__TAURI__.invoke。
interface Window {
  __TAURI__?: {
    invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T>;
  };
}
