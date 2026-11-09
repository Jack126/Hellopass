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

const DEFAULT_ACCEL: &str = "control+alt+space";

#[derive(Clone, serde::Serialize)]
struct HotkeyInfo {
    accelerator: String,
    display: String,
}

struct HotkeyStateInner {
    shortcut: Shortcut,
    display: String,
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

fn parse_hotkey(accel: &str) -> Result<Shortcut, String> {
    let sc: Shortcut = accel
        .trim()
        .parse()
        .map_err(|e| format!("无法识别的快捷键: {}", e))?;
    let label = key_label(&sc.key);
    let is_fkey =
        label.len() > 1 && label.starts_with('F') && label[1..].chars().all(|c| c.is_ascii_digit());
    if sc.mods.is_empty() && !is_fkey {
        return Err("快捷键至少要带 Ctrl / Alt / Shift 之一（F1~F24 除外）".to_string());
    }
    Ok(sc)
}

fn info_of(sc: &Shortcut) -> HotkeyInfo {
    HotkeyInfo {
        accelerator: format!("{}", sc),
        display: hotkey_display(sc),
    }
}

fn load_saved_accel(path: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("accelerator")?.as_str().map(|s| s.to_string())
}

fn save_accel(path: &std::path::Path, accel: &str) {
    let body = serde_json::json!({ "accelerator": accel }).to_string();
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
fn lock(state: State<'_, VaultState>) -> ApiResponse<()> {
    let mut g = state.lock().unwrap();
    g.lock();
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

/// 30 秒后只在剪贴板内容仍是我们写入的那份时才清空，避免误删用户后来复制的内容。
fn schedule_clipboard_clear(app: AppHandle, secret: &str) {
    let digest = secret_digest(secret);
    std::thread::spawn(move || {
        std::thread::sleep(StdDuration::from_secs(30));
        if let Ok(current) = app.clipboard().read_text() {
            if secret_digest(&current) == digest {
                let _ = app.clipboard().clear();
            }
        }
    });
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
    schedule_clipboard_clear(app, secret.as_str());
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
    ApiResponse::ok(info_of(&g.shortcut))
}

/// 开始录制：先注销当前热键，否则系统会把组合键截走，录制窗口收不到按键。
#[tauri::command]
fn begin_hotkey_capture(app: AppHandle, state: State<'_, HotkeyState>) -> ApiResponse<()> {
    let mut g = state.0.lock().unwrap();
    let _ = app.global_shortcut().unregister(g.shortcut.clone());
    g.suspended = true;
    ApiResponse::ok(())
}

#[tauri::command]
fn end_hotkey_capture(app: AppHandle, state: State<'_, HotkeyState>) -> ApiResponse<()> {
    let mut g = state.0.lock().unwrap();
    if g.suspended {
        if let Err(e) = app.global_shortcut().register(g.shortcut.clone()) {
            crate::utils::log::error("hotkey", &format!("恢复热键失败: {}", e));
        }
        g.suspended = false;
    }
    ApiResponse::ok(())
}

#[tauri::command]
fn set_hotkey(
    app: AppHandle,
    state: State<'_, HotkeyState>,
    accelerator: String,
) -> ApiResponse<HotkeyInfo> {
    let new = match parse_hotkey(&accelerator) {
        Ok(s) => s,
        Err(e) => return ApiResponse::err_message(e),
    };
    let mut g = state.0.lock().unwrap();
    let old = g.shortcut.clone();
    let _ = app.global_shortcut().unregister(old.clone());
    match app.global_shortcut().register(new.clone()) {
        Ok(_) => {
            g.shortcut = new.clone();
            g.display = hotkey_display(&new);
            g.suspended = false;
            save_accel(&g.path, &format!("{}", new));
            ApiResponse::ok(info_of(&new))
        }
        Err(e) => {
            // 注册失败就把旧热键恢复回去，避免应用失去唤出方式
            if let Err(restore) = app.global_shortcut().register(old.clone()) {
                crate::utils::log::error("hotkey", &format!("恢复旧热键失败: {}", restore));
            }
            g.suspended = false;
            ApiResponse::err_message(format!("注册失败：{}（可能被其他程序占用）", e))
        }
    }
}

/// 调整启动器高度：搜索 420，详情/编辑 560。
#[tauri::command]
fn resize_launcher(app: AppHandle, height: f64) -> ApiResponse<()> {
    if let Some(w) = app.get_webview_window("launcher") {
        let _ = w.set_size(tauri::LogicalSize::new(680.0, height));
        let _ = w.center();
    }
    ApiResponse::ok(())
}

fn show_launcher(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("launcher") {
        let _ = w.center();
        let _ = w.show();
        let _ = w.set_focus();
        let _ = app.emit("launcher-shown", ());
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
                        let _ = handle.global_shortcut().register(g.shortcut.clone());
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
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        toggle_launcher(app);
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

            // 管理保险库运行时（Mutex 保证 Send+Sync）
            app.manage(Mutex::new(VaultRuntime::new(config_dir, db_path)));
            app.manage(LauncherFlags::default());

            // 启动器窗口：常驻但不显示，热键唤出
            let _launcher = tauri::WebviewWindowBuilder::new(
                app,
                "launcher",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("SecurePass")
            .inner_size(680.0, 420.0)
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

            // 全局热键：优先用设置里保存的组合，注册失败则退回默认值
            let default_shortcut = parse_hotkey(DEFAULT_ACCEL).expect("默认热键必须可解析");
            let saved = load_saved_accel(&hotkey_path).unwrap_or_else(|| DEFAULT_ACCEL.to_string());
            let mut chosen = parse_hotkey(&saved).unwrap_or_else(|_| default_shortcut.clone());
            if let Err(e) = app.global_shortcut().register(chosen.clone()) {
                crate::utils::log::error(
                    "hotkey",
                    &format!("注册热键 {} 失败: {}", hotkey_display(&chosen), e),
                );
                if chosen != default_shortcut {
                    chosen = default_shortcut.clone();
                    if let Err(e2) = app.global_shortcut().register(chosen.clone()) {
                        crate::utils::log::error(
                            "hotkey",
                            &format!("注册默认热键失败: {}", e2),
                        );
                    }
                }
            }
            app.manage(HotkeyState(Mutex::new(HotkeyStateInner {
                display: hotkey_display(&chosen),
                shortcut: chosen,
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
                    "lock" => {
                        if let Some(state) = app.try_state::<VaultState>() {
                            state.lock().unwrap().lock();
                        }
                        let _ = app.emit("vault-locked", ());
                        if let Some(w) = app.get_webview_window("launcher") {
                            let _ = w.hide();
                        }
                    }
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
            hide_launcher,
            set_suppress_hide,
            get_hotkey,
            set_hotkey,
            begin_hotkey_capture,
            end_hotkey_capture
        ])
        .build(tauri::generate_context!())
        .expect("初始化 SecurePass 失败")
        .run(|app, event| {
            // 任何退出路径都先锁定，保证基线是最新的
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(state) = app.try_state::<VaultState>() {
                    state.lock().unwrap().lock();
                }
            }
        });
}
