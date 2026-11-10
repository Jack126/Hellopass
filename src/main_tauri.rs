// Release 下用窗口子系统，避免启动时弹出一个控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod security;
mod database;
mod api;
mod utils;
mod error;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration as StdDuration;

use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use crate::api::runtime::VaultRuntime;
use crate::api::dto::*;
use crate::security::policies::PasswordStrengthResult;

type VaultState = Mutex<VaultRuntime>;

/// 启动器的临时开关：口令输入/编辑时暂停「失焦自动收起」。
#[derive(Default)]
struct LauncherFlags {
    suppress_hide: AtomicBool,
}

/// 窗口大小偏好（auto / compact / standard / large），落盘到 config/ui.json。
struct UiPrefs {
    preset: Mutex<String>,
    path: std::path::PathBuf,
}

fn load_ui_preset(path: &std::path::Path) -> String {
    let Ok(text) = std::fs::read_to_string(path) else {
        return "auto".to_string();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return "auto".to_string();
    };
    value
        .get("window_preset")
        .and_then(|v| v.as_str())
        .filter(|s| crate::utils::window_size::is_valid_preset(s))
        .unwrap_or("auto")
        .to_string()
}

fn save_ui_preset(path: &std::path::Path, preset: &str) {
    let body = serde_json::json!({ "window_preset": preset }).to_string();
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, body).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

fn monitor_under_cursor(app: &AppHandle) -> Option<tauri::Monitor> {
    let pos = app.cursor_position().ok()?;
    app.monitor_from_point(pos.x, pos.y).ok().flatten()
}

/// 按当前偏好和显示器计算尺寸；expanded=详情/编辑态，place=把窗口摆到鼠标所在屏幕中央。
fn apply_launcher_size(app: &AppHandle, expanded: bool, place: bool) {
    let preset = app
        .try_state::<UiPrefs>()
        .map(|s| s.preset.lock().unwrap().clone())
        .unwrap_or_else(|| "auto".to_string());

    let monitor = if place {
        monitor_under_cursor(app)
    } else {
        app.get_webview_window("launcher")
            .and_then(|w| w.current_monitor().ok().flatten())
            .or_else(|| monitor_under_cursor(app))
    };

    // 用物理像素计算，避免多显示器不同缩放比时 LogicalPosition 换算错位
    let (phys_w, phys_h, pos_x, pos_y, scale) = match monitor {
        Some(m) => (
            m.size().width as f64,
            m.size().height as f64,
            m.position().x as f64,
            m.position().y as f64,
            m.scale_factor(),
        ),
        None => (1920.0, 1080.0, 0.0, 0.0, 1.0),
    };
    let monitor_w = phys_w / scale;
    let monitor_h = phys_h / scale;

    let computed = crate::utils::window_size::compute(&preset, monitor_w, monitor_h);
    let width = computed.width;
    let height = if expanded {
        computed.tall_height
    } else {
        computed.base_height
    };
    let width_px = (width * scale).round() as u32;
    let height_px = (height * scale).round() as u32;

    if let Some(w) = app.get_webview_window("launcher") {
        let _ = w.set_size(tauri::PhysicalSize::new(width_px, height_px));
        if place {
            let x = pos_x + (phys_w - width_px as f64) / 2.0;
            let y = pos_y + (phys_h - height_px as f64) / 2.0;
            let _ = w.set_position(tauri::PhysicalPosition::new(x.round() as i32, y.round() as i32));
        } else {
            let _ = w.center();
        }
    }
}

/// 记录最近一次写进剪贴板的密码摘要，锁定时只在内容仍是它时才清空。
#[derive(Default)]
struct ClipboardGuard(Mutex<Option<String>>);

const DEFAULT_WAKE_ACCEL: &str = "control+alt+space";
const DEFAULT_LOCK_ACCEL: &str = "control+shift+l";
// 插件用 id 作为注册表键，两个热键必须用不同 id，否则后注册的会覆盖前者。
const WAKE_HOTKEY_ID: u32 = 1;
const LOCK_HOTKEY_ID: u32 = 2;

#[derive(Clone, serde::Serialize)]
struct HotkeyInfo {
    wake: String,
    wake_display: String,
    lock: String,
    lock_display: String,
}

struct HotkeyStateInner {
    wake: Shortcut,
    wake_display: String,
    lock: Shortcut,
    lock_display: String,
    suspended: bool,
    path: std::path::PathBuf,
}

struct HotkeyState(Mutex<HotkeyStateInner>);

fn key_label(code: &Code) -> String {
    let s = code.to_string();
    if let Some(rest) = s.strip_prefix("Key") {
        return rest.to_string();
    }
    if let Some(rest) = s.strip_prefix("Digit") {
        return rest.to_string();
    }
    if let Some(rest) = s.strip_prefix("Numpad") {
        return format!("Num{}", rest);
    }
    s
}

fn hotkey_display(sc: &Shortcut) -> String {
    let mut parts: Vec<String> = Vec::new();
    if sc.mods.contains(Modifiers::SHIFT) {
        parts.push("Shift".into());
    }
    if sc.mods.contains(Modifiers::CONTROL) {
        parts.push("Ctrl".into());
    }
    if sc.mods.contains(Modifiers::ALT) {
        parts.push("Alt".into());
    }
    if sc.mods.contains(Modifiers::SUPER) {
        parts.push("Win".into());
    }
    parts.push(key_label(&sc.key));
    parts.join(" + ")
}

fn parse_hotkey(accel: &str, id: u32) -> Result<Shortcut, String> {
    let mut sc: Shortcut = accel
        .trim()
        .parse()
        .map_err(|e| format!("无法识别的快捷键: {}", e))?;
    sc.id = id;
    let label = key_label(&sc.key);
    let is_fkey =
        label.len() > 1 && label.starts_with('F') && label[1..].chars().all(|c| c.is_ascii_digit());
    if sc.mods.is_empty() && !is_fkey {
        return Err("快捷键至少要带 Ctrl / Alt / Shift 之一（F1~F24 除外）".to_string());
    }
    Ok(sc)
}

fn info_of(wake: &Shortcut, lock: &Shortcut) -> HotkeyInfo {
    HotkeyInfo {
        wake: format!("{}", wake),
        wake_display: hotkey_display(wake),
        lock: format!("{}", lock),
        lock_display: hotkey_display(lock),
    }
}

fn load_saved_accels(path: &std::path::Path) -> (Option<String>, Option<String>) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return (None, None);
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return (None, None);
    };
    (
        value.get("accelerator").and_then(|v| v.as_str()).map(|s| s.to_string()),
        value
            .get("lock_accelerator")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    )
}

fn save_accels(path: &std::path::Path, wake: &str, lock: &str) {
    let body = serde_json::json!({
        "accelerator": wake,
        "lock_accelerator": lock,
    })
    .to_string();
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, body).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

#[tauri::command]
fn setup_master_password(state: State<'_, VaultState>, master_password: String) -> ApiResponse<()> {
    let mut g = state.lock().unwrap();
    g.setup(&master_password)
}

#[tauri::command]
fn is_master_password_set(state: State<'_, VaultState>) -> ApiResponse<bool> {
    let g = state.lock().unwrap();
    ApiResponse::ok(g.is_set())
}

#[tauri::command]
fn unlock(state: State<'_, VaultState>, master_password: String) -> ApiResponse<()> {
    let mut g = state.lock().unwrap();
    g.unlock(&master_password)
}

#[tauri::command]
fn lock(app: AppHandle, state: State<'_, VaultState>) -> ApiResponse<()> {
    let mut g = state.lock().unwrap();
    g.lock();
    clear_clipboard_if_ours(&app);
    ApiResponse::ok(())
}

#[tauri::command]
fn change_master_password(
    state: State<'_, VaultState>,
    old_password: String,
    new_password: String,
) -> ApiResponse<()> {
    let mut g = state.lock().unwrap();
    g.change_password(&old_password, &new_password)
}

#[tauri::command]
fn add_password(state: State<'_, VaultState>, password: NewPassword) -> ApiResponse<PasswordEntry> {
    let mut g = state.lock().unwrap();
    g.add_password(password)
}

#[tauri::command]
fn get_passwords(state: State<'_, VaultState>) -> ApiResponse<Vec<PasswordEntry>> {
    let mut g = state.lock().unwrap();
    g.get_passwords()
}

#[tauri::command]
fn get_password(state: State<'_, VaultState>, id: i64) -> ApiResponse<PasswordEntry> {
    let mut g = state.lock().unwrap();
    g.get_password(id)
}

#[tauri::command]
fn search_passwords(state: State<'_, VaultState>, query: SearchQuery) -> ApiResponse<Vec<PasswordEntry>> {
    let mut g = state.lock().unwrap();
    g.search_passwords(query)
}

/// 启动器检索用摘要：只含元数据，不含任何明文/密文密码。
#[tauri::command]
fn list_entry_summaries(state: State<'_, VaultState>) -> ApiResponse<Vec<EntrySummary>> {
    let mut g = state.lock().unwrap();
    g.list_entry_summaries()
}

/// 当前是否已解锁（启动器唤出时用它决定显示搜索还是解锁）。
#[tauri::command]
fn vault_status(state: State<'_, VaultState>) -> ApiResponse<bool> {
    let mut g = state.lock().unwrap();
    ApiResponse::ok(g.is_unlocked())
}

fn secret_digest(secret: &str) -> String {
    let mut h = Sha256::new();
    h.update(secret.as_bytes());
    format!("{:x}", h.finalize())
}

/// 30 秒后清空；若期间剪贴板被用户改过、或已因锁定清空，则不动。
fn schedule_clipboard_clear(app: AppHandle, digest: String) {
    std::thread::spawn(move || {
        std::thread::sleep(StdDuration::from_secs(30));
        let Some(guard) = app.try_state::<ClipboardGuard>() else {
            return;
        };
        let still_ours = {
            let mut g = guard.0.lock().unwrap();
            if g.as_deref() == Some(digest.as_str()) {
                *g = None;
                true
            } else {
                false
            }
        };
        if !still_ours {
            return;
        }
        if let Ok(current) = app.clipboard().read_text() {
            if secret_digest(&current) == digest {
                let _ = app.clipboard().clear();
            }
        }
    });
}

/// 剪贴板里若仍是我们复制的密码，立即清空（锁定/退出时调用）。
fn clear_clipboard_if_ours(app: &AppHandle) {
    let Some(guard) = app.try_state::<ClipboardGuard>() else {
        return;
    };
    let digest = { guard.0.lock().unwrap().take() };
    let Some(digest) = digest else {
        return;
    };
    if let Ok(current) = app.clipboard().read_text() {
        if secret_digest(&current) == digest {
            let _ = app.clipboard().clear();
        }
    }
}

/// 复制密码/用户名：解密、写剪贴板、定时清理都在 Rust 侧完成，明文不经过前端。
#[tauri::command]
fn copy_secret(
    app: AppHandle,
    state: State<'_, VaultState>,
    id: i64,
    field: String,
) -> ApiResponse<()> {
    let secret = {
        let mut g = state.lock().unwrap();
        match g.secret_for_copy(id, &field) {
            Ok(s) => s,
            Err(e) => return ApiResponse::err_message(e.to_string()),
        }
    };
    if let Err(e) = app.clipboard().write_text(secret.as_str()) {
        return ApiResponse::err_message(format!("写入剪贴板失败: {}", e));
    }
    let digest = secret_digest(secret.as_str());
    if let Some(guard) = app.try_state::<ClipboardGuard>() {
        *guard.0.lock().unwrap() = Some(digest.clone());
    }
    schedule_clipboard_clear(app, digest);
    ApiResponse::ok(())
}

/// 详情面板数据（不含密码）。
#[tauri::command]
fn get_entry_detail(state: State<'_, VaultState>, id: i64) -> ApiResponse<EntryDetail> {
    let mut g = state.lock().unwrap();
    match g.entry_detail(id) {
        Ok(d) => ApiResponse::ok(d),
        Err(e) => ApiResponse::err_message(e.to_string()),
    }
}

/// 详情页显式查看密码：唯一向界面返回明文密码的入口。
#[tauri::command]
fn reveal_password(state: State<'_, VaultState>, id: i64) -> ApiResponse<String> {
    let mut g = state.lock().unwrap();
    match g.reveal_password(id) {
        Ok(s) => ApiResponse::ok(s.to_string()),
        Err(e) => ApiResponse::err_message(e.to_string()),
    }
}

#[tauri::command]
fn hide_launcher(app: AppHandle) -> ApiResponse<()> {
    if let Some(w) = app.get_webview_window("launcher") {
        let _ = w.hide();
        let _ = app.emit("launcher-hidden", ());
    }
    ApiResponse::ok(())
}

#[tauri::command]
fn set_suppress_hide(app: AppHandle, suppress: bool) -> ApiResponse<()> {
    if let Some(flags) = app.try_state::<LauncherFlags>() {
        flags.suppress_hide.store(suppress, Ordering::Relaxed);
    }
    ApiResponse::ok(())
}

#[tauri::command]
fn get_hotkey(state: State<'_, HotkeyState>) -> ApiResponse<HotkeyInfo> {
    let g = state.0.lock().unwrap();
    ApiResponse::ok(info_of(&g.wake, &g.lock))
}

/// 开始录制：两个热键都先注销，否则系统会把组合键截走。
#[tauri::command]
fn begin_hotkey_capture(app: AppHandle, state: State<'_, HotkeyState>) -> ApiResponse<()> {
    let mut g = state.0.lock().unwrap();
    let _ = app.global_shortcut().unregister(g.wake.clone());
    let _ = app.global_shortcut().unregister(g.lock.clone());
    g.suspended = true;
    ApiResponse::ok(())
}

#[tauri::command]
fn end_hotkey_capture(app: AppHandle, state: State<'_, HotkeyState>) -> ApiResponse<()> {
    let mut g = state.0.lock().unwrap();
    if g.suspended {
        if let Err(e) = app.global_shortcut().register(g.wake.clone()) {
            crate::utils::log::error("hotkey", &format!("恢复唤醒热键失败: {}", e));
        }
        if let Err(e) = app.global_shortcut().register(g.lock.clone()) {
            crate::utils::log::error("hotkey", &format!("恢复锁定热键失败: {}", e));
        }
        g.suspended = false;
    }
    ApiResponse::ok(())
}

#[derive(Clone, serde::Serialize)]
struct HotkeyCheck {
    available: bool,
    reason: Option<String>,
}

/// 录制阶段检查组合键是否可用：与另一个热键重复，或被系统/其他程序注册。
#[tauri::command]
fn check_hotkey_available(
    app: AppHandle,
    state: State<'_, HotkeyState>,
    kind: String,
    accelerator: String,
) -> ApiResponse<HotkeyCheck> {
    if kind != "wake" && kind != "lock" {
        return ApiResponse::err_message("未知的热键类型");
    }
    let hotkey_id = if kind == "lock" { LOCK_HOTKEY_ID } else { WAKE_HOTKEY_ID };
    let parsed = match parse_hotkey(&accelerator, hotkey_id) {
        Ok(s) => s,
        Err(e) => return ApiResponse::err_message(e),
    };
    let (other, suspended) = {
        let g = state.0.lock().unwrap();
        let other = if kind == "lock" {
            g.wake.clone()
        } else {
            g.lock.clone()
        };
        (other, g.suspended)
    };
    if parsed == other {
        return ApiResponse::ok(HotkeyCheck {
            available: false,
            reason: Some("与另一个热键相同".to_string()),
        });
    }
    if !suspended {
        // 当前两个热键都注册着，无法探测，交给真正的保存流程报错
        return ApiResponse::ok(HotkeyCheck {
            available: true,
            reason: None,
        });
    }
    match app.global_shortcut().register(parsed.clone()) {
        Ok(_) => {
            let _ = app.global_shortcut().unregister(parsed);
            ApiResponse::ok(HotkeyCheck {
                available: true,
                reason: None,
            })
        }
        Err(e) => ApiResponse::ok(HotkeyCheck {
            available: false,
            reason: Some(format!("已被其他程序占用：{}", e)),
        }),
    }
}

#[tauri::command]
fn set_hotkey(
    app: AppHandle,
    state: State<'_, HotkeyState>,
    kind: String,
    accelerator: String,
) -> ApiResponse<HotkeyInfo> {
    if kind != "wake" && kind != "lock" {
        return ApiResponse::err_message("未知的热键类型");
    }
    let hotkey_id = if kind == "lock" { LOCK_HOTKEY_ID } else { WAKE_HOTKEY_ID };
    let new = match parse_hotkey(&accelerator, hotkey_id) {
        Ok(s) => s,
        Err(e) => return ApiResponse::err_message(e),
    };
    let mut g = state.0.lock().unwrap();
    let (old, other) = if kind == "lock" {
        (g.lock.clone(), g.wake.clone())
    } else {
        (g.wake.clone(), g.lock.clone())
    };
    if new == other {
        return ApiResponse::err_message("不能与另一个热键相同");
    }
    let _ = app.global_shortcut().unregister(old.clone());
    match app.global_shortcut().register(new.clone()) {
        Ok(_) => {
            if kind == "lock" {
                g.lock = new.clone();
                g.lock_display = hotkey_display(&new);
            } else {
                g.wake = new.clone();
                g.wake_display = hotkey_display(&new);
            }
            if g.suspended {
                let _ = app.global_shortcut().register(other);
                g.suspended = false;
            }
            save_accels(&g.path, &format!("{}", g.wake), &format!("{}", g.lock));
            ApiResponse::ok(info_of(&g.wake, &g.lock))
        }
        Err(e) => {
            if let Err(restore) = app.global_shortcut().register(old.clone()) {
                crate::utils::log::error("hotkey", &format!("恢复旧热键失败: {}", restore));
            }
            if g.suspended {
                let _ = app.global_shortcut().register(other);
                g.suspended = false;
            }
            ApiResponse::err_message(format!("注册失败：{}（可能被其他程序占用）", e))
        }
    }
}

/// expanded=true 用于详情/编辑态（更高的窗口）。
#[tauri::command]
fn resize_launcher(app: AppHandle, expanded: bool) -> ApiResponse<()> {
    apply_launcher_size(&app, expanded, false);
    ApiResponse::ok(())
}

#[tauri::command]
fn get_window_preset(state: State<'_, UiPrefs>) -> ApiResponse<String> {
    ApiResponse::ok(state.preset.lock().unwrap().clone())
}

#[tauri::command]
fn set_window_preset(
    app: AppHandle,
    state: State<'_, UiPrefs>,
    preset: String,
) -> ApiResponse<()> {
    if !crate::utils::window_size::is_valid_preset(&preset) {
        return ApiResponse::err_message("未知的窗口大小");
    }
    *state.preset.lock().unwrap() = preset.clone();
    save_ui_preset(&state.path, &preset);
    apply_launcher_size(&app, false, false);
    ApiResponse::ok(())
}

fn show_launcher(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("launcher") {
        apply_launcher_size(app, false, true);
        let _ = w.show();
        let _ = w.set_focus();
        let _ = app.emit("launcher-shown", ());
        // 后端兜底：锁定时再发一次锁定事件，前端即便没查到状态也会回到解锁界面
        let locked = app
            .try_state::<VaultState>()
            .map(|s| !s.lock().unwrap().is_unlocked())
            .unwrap_or(true);
        if locked {
            let _ = app.emit("vault-locked", ());
        }
    }
}

fn toggle_launcher(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("launcher") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            show_launcher(app);
        }
    }
}

fn lock_vault(app: &AppHandle) {
    if let Some(state) = app.try_state::<VaultState>() {
        state.lock().unwrap().lock();
    }
    clear_clipboard_if_ours(app);
    let _ = app.emit("vault-locked", ());
    if let Some(w) = app.get_webview_window("launcher") {
        let _ = w.hide();
    }
    // 设置窗里有库内容/审计信息，锁定后直接关掉
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.close();
    }
}

/// 注册单个热键：优先用保存值，失败退回默认值。
fn register_hotkey_with_fallback(
    app: &AppHandle,
    saved: Option<String>,
    default_accel: &str,
    label: &str,
    id: u32,
) -> Shortcut {
    let default = parse_hotkey(default_accel, id).expect("默认热键必须可解析");
    let mut chosen = saved
        .and_then(|s| parse_hotkey(&s, id).ok())
        .unwrap_or_else(|| default.clone());
    if let Err(e) = app.global_shortcut().register(chosen.clone()) {
        crate::utils::log::error(
            "hotkey",
            &format!("注册{}热键 {} 失败: {}", label, hotkey_display(&chosen), e),
        );
        if chosen != default {
            chosen = default.clone();
            if let Err(e2) = app.global_shortcut().register(chosen.clone()) {
                crate::utils::log::error(
                    "hotkey",
                    &format!("注册默认{}热键失败: {}", label, e2),
                );
            }
        }
    }
    chosen
}

fn open_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    if let Ok(win) = tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        tauri::WebviewUrl::App("setting.html".into()),
    )
    .title("SecurePass 设置")
    .inner_size(720.0, 600.0)
    .min_inner_size(640.0, 520.0)
    .background_color(tauri::window::Color(24, 24, 27, 255))
    .build()
    {
        // 录制热键时旧热键被注销；若用户直接关窗，这里负责恢复，避免应用失去唤出方式
        let handle = app.clone();
        let _ = win.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Some(state) = handle.try_state::<HotkeyState>() {
                    let mut g = state.0.lock().unwrap();
                    if g.suspended {
                        let _ = handle.global_shortcut().register(g.wake.clone());
                        let _ = handle.global_shortcut().register(g.lock.clone());
                        g.suspended = false;
                    }
                }
            }
        });
    }
}

#[tauri::command]
fn update_password(state: State<'_, VaultState>, password: UpdatePassword) -> ApiResponse<PasswordEntry> {
    let mut g = state.lock().unwrap();
    g.update_password(password)
}

#[tauri::command]
fn delete_password(state: State<'_, VaultState>, id: i64) -> ApiResponse<()> {
    let mut g = state.lock().unwrap();
    g.delete_password(id)
}

#[tauri::command]
fn get_password_stats(state: State<'_, VaultState>) -> ApiResponse<PasswordStats> {
    let mut g = state.lock().unwrap();
    g.get_password_stats()
}

#[tauri::command]
fn import_passwords(state: State<'_, VaultState>, data: String) -> ApiResponse<ImportResult> {
    let mut g = state.lock().unwrap();
    g.import_passwords(data)
}

#[tauri::command]
fn export_backup(state: State<'_, VaultState>) -> ApiResponse<String> {
    let mut g = state.lock().unwrap();
    g.export_backup()
}

#[tauri::command]
fn restore_backup(state: State<'_, VaultState>, data: String, master_password: String) -> ApiResponse<ImportResult> {
    let mut g = state.lock().unwrap();
    g.restore_backup(data, master_password)
}

#[tauri::command]
fn export_encrypted(
    state: State<'_, VaultState>,
    export_password: String,
    ids: Option<Vec<i64>>,
) -> ApiResponse<String> {
    let mut g = state.lock().unwrap();
    g.export_encrypted(&export_password, ids)
}

#[tauri::command]
fn import_encrypted(state: State<'_, VaultState>, data: String, export_password: String) -> ApiResponse<ImportResult> {
    let mut g = state.lock().unwrap();
    g.import_encrypted(data, export_password)
}

/// 把导出的文件内容写入「下载」目录并返回完整路径。
///
/// WebView 里的 <a download> 在 WebKitGTK 下不会触发任何下载（点了没反应），
/// 因此导出/备份统一走后端落盘。
#[tauri::command]
fn save_export_file(
    app: tauri::AppHandle,
    filename: String,
    content: String,
) -> ApiResponse<String> {
    let safe_name = std::path::Path::new(&filename)
        .file_name()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| std::ffi::OsString::from("securepass-export.bin"));

    let dir = app
        .path()
        .download_dir()
        .ok()
        .filter(|d| d.exists())
        .or_else(|| app.path().home_dir().ok())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let path = dir.join(safe_name);
    match std::fs::write(&path, content.as_bytes()) {
        Ok(_) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&path) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o600);
                    let _ = std::fs::set_permissions(&path, perms);
                }
            }
            ApiResponse::ok(path.display().to_string())
        }
        Err(e) => ApiResponse::err_message(format!("保存文件失败: {}", e)),
    }
}

/// 前端「自动锁定时间」设置同步给后端会话超时。
#[tauri::command]
fn set_session_timeout(state: State<'_, VaultState>, seconds: i64) -> ApiResponse<()> {
    let mut g = state.lock().unwrap();
    g.set_session_timeout(seconds);
    ApiResponse::ok(())
}

#[tauri::command]
fn check_password_strength(state: State<'_, VaultState>, password: String) -> ApiResponse<PasswordStrengthResult> {
    let g = state.lock().unwrap();
    g.check_password_strength(&password)
}

#[tauri::command]
fn get_password_policy(state: State<'_, VaultState>) -> ApiResponse<PasswordPolicy> {
    let g = state.lock().unwrap();
    g.get_password_policy()
}

#[tauri::command]
fn get_expiring_passwords(state: State<'_, VaultState>, within_days: i64) -> ApiResponse<Vec<PasswordExpiryInfo>> {
    let mut g = state.lock().unwrap();
    g.get_expiring_passwords(within_days)
}

#[tauri::command]
fn get_audit_logs(state: State<'_, VaultState>) -> ApiResponse<Vec<AuditLogEntry>> {
    let g = state.lock().unwrap();
    g.get_audit_logs()
}

#[tauri::command]
fn purge_old_audit_logs(state: State<'_, VaultState>, days: i64) -> ApiResponse<usize> {
    let mut g = state.lock().unwrap();
    g.purge_old_audit_logs(days)
}

#[tauri::command]
fn check_integrity(state: State<'_, VaultState>) -> ApiResponse<IntegrityReport> {
    let mut g = state.lock().unwrap();
    g.check_integrity()
}

#[tauri::command]
fn rebuild_baseline(state: State<'_, VaultState>) -> ApiResponse<()> {
    let mut g = state.lock().unwrap();
    g.rebuild_baseline()
}

fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let Some(state) = app.try_state::<HotkeyState>() else {
                        return;
                    };
                    let (is_wake, is_lock) = {
                        let g = state.0.lock().unwrap();
                        (g.wake == *shortcut, g.lock == *shortcut)
                    };
                    if is_wake {
                        toggle_launcher(app);
                    } else if is_lock {
                        lock_vault(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            // 解析应用数据目录（跨平台）
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("无法解析应用数据目录");
            std::fs::create_dir_all(&app_dir).ok();
            let config_dir = app_dir.join("config");
            std::fs::create_dir_all(&config_dir).ok();
            // 数据库位置：
            // - 开发构建：固定用仓库根目录的 securepass.db，方便直接用 sqlite3 查看/备份；
            // - 发布构建：仍用系统 app data 目录，避免安装目录只读导致无法写入。
            #[cfg(debug_assertions)]
            let db_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("securepass.db");
            #[cfg(not(debug_assertions))]
            let db_path = app_dir.join("securepass.db");

            let hotkey_path = config_dir.join("hotkey.json");
            let ui_path = config_dir.join("ui.json");
            let ui_preset = load_ui_preset(&ui_path);
            let initial_size = {
                let h = app.handle().clone();
                let (mw, mh) = monitor_under_cursor(&h)
                    .map(|m| {
                        let s = m.scale_factor();
                        let sz = m.size().to_logical::<f64>(s);
                        (sz.width, sz.height)
                    })
                    .unwrap_or((1920.0, 1080.0));
                crate::utils::window_size::compute(&ui_preset, mw, mh)
            };

            // 管理保险库运行时（Mutex 保证 Send+Sync）
            app.manage(Mutex::new(VaultRuntime::new(config_dir, db_path)));
            app.manage(LauncherFlags::default());
            app.manage(ClipboardGuard::default());
            app.manage(UiPrefs {
                preset: Mutex::new(ui_preset),
                path: ui_path,
            });

            // 启动器窗口：常驻但不显示，热键唤出
            let _launcher = tauri::WebviewWindowBuilder::new(
                app,
                "launcher",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("SecurePass")
            .inner_size(initial_size.width, initial_size.base_height)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .center()
            .build()
            .expect("创建启动器窗口失败");

            // 失焦自动收起（口令输入/编辑时由前端暂停）
            let handle = app.handle().clone();
            let _ = _launcher.on_window_event(move |event| match event {
                // 启动器窗口永不销毁，Alt+F4 只收起
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    if let Some(w) = handle.get_webview_window("launcher") {
                        let _ = w.hide();
                    }
                }
                tauri::WindowEvent::Focused(false) => {
                    if let Some(flags) = handle.try_state::<LauncherFlags>() {
                        if flags.suppress_hide.load(Ordering::Relaxed) {
                            return;
                        }
                    }
                    if let Some(w) = handle.get_webview_window("launcher") {
                        let _ = w.hide();
                        let _ = handle.emit("launcher-hidden", ());
                    }
                }
                _ => {}
            });

            // 全局热键：唤醒 / 锁定各一个，优先用保存值，失败退回默认值
            let (saved_wake, mut saved_lock) = load_saved_accels(&hotkey_path);
            // 旧默认 Ctrl+Alt+L 会被 QQ 抢走，检测到旧默认值时迁移到新默认
            if let Some(s) = &saved_lock {
                if s.eq_ignore_ascii_case("control+alt+l") {
                    saved_lock = None;
                }
            }
            let handle = app.handle().clone();
            let wake = register_hotkey_with_fallback(
                &handle,
                saved_wake,
                DEFAULT_WAKE_ACCEL,
                "唤醒",
                WAKE_HOTKEY_ID,
            );
            let lock = register_hotkey_with_fallback(
                &handle,
                saved_lock,
                DEFAULT_LOCK_ACCEL,
                "锁定",
                LOCK_HOTKEY_ID,
            );
            app.manage(HotkeyState(Mutex::new(HotkeyStateInner {
                wake_display: hotkey_display(&wake),
                wake,
                lock_display: hotkey_display(&lock),
                lock,
                suspended: false,
                path: hotkey_path,
            })));

            // 系统托盘菜单
            use tauri::menu::{MenuBuilder, MenuItem, PredefinedMenuItem};
            let search = MenuItem::with_id(app, "search", "搜索", true, None::<&str>)
                .expect("创建菜单项失败");
            let lock = MenuItem::with_id(app, "lock", "立即锁定", true, None::<&str>)
                .expect("创建菜单项失败");
            let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)
                .expect("创建菜单项失败");
            let quit = MenuItem::with_id(app, "quit", "退出 SecurePass", true, None::<&str>)
                .expect("创建菜单项失败");
            let sep1 = PredefinedMenuItem::separator(app).expect("创建分隔符失败");
            let sep2 = PredefinedMenuItem::separator(app).expect("创建分隔符失败");
            let menu = MenuBuilder::new(app)
                .items(&[&search, &lock, &sep1, &settings, &sep2, &quit])
                .build()
                .expect("创建菜单失败");

            let _tray = tauri::tray::TrayIconBuilder::with_id("securepass-tray")
                .icon(
                    app.default_window_icon()
                        .expect("缺少默认窗口图标")
                        .clone(),
                )
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "search" => show_launcher(app),
                    "lock" => lock_vault(app),
                    "settings" => open_settings(app),
                    "quit" => {
                        // 退出前正常锁定：写入完整性基线，避免下次解锁误报「不一致」
                        if let Some(state) = app.try_state::<VaultState>() {
                            state.lock().unwrap().lock();
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        show_launcher(tray.app_handle());
                    }
                })
                .build(app)
                .expect("创建托盘失败");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            setup_master_password,
            is_master_password_set,
            unlock,
            lock,
            change_master_password,
            add_password,
            get_passwords,
            get_password,
            search_passwords,
            update_password,
            delete_password,
            get_password_stats,
            import_passwords,
            set_session_timeout,
            save_export_file,
            export_backup,
            restore_backup,
            export_encrypted,
            import_encrypted,
            get_audit_logs,
            purge_old_audit_logs,
            check_integrity,
            rebuild_baseline,
            check_password_strength,
            get_password_policy,
            get_expiring_passwords,
            list_entry_summaries,
            vault_status,
            copy_secret,
            reveal_password,
            get_entry_detail,
            resize_launcher,
            get_window_preset,
            set_window_preset,
            hide_launcher,
            set_suppress_hide,
            get_hotkey,
            set_hotkey,
            begin_hotkey_capture,
            end_hotkey_capture,
            check_hotkey_available
        ])
        .build(tauri::generate_context!())
        .expect("初始化 SecurePass 失败")
        .run(|app, event| {
            // 任何退出路径都先锁定，保证基线是最新的
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(state) = app.try_state::<VaultState>() {
                    state.lock().unwrap().lock();
                }
                clear_clipboard_if_ours(app);
            }
        });
}
