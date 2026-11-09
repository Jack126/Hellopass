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
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_notification::NotificationExt;
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

fn notify(app: &AppHandle, body: &str) {
    if let Err(e) = app
        .notification()
        .builder()
        .title("SecurePass")
        .body(body)
        .show()
    {
        crate::utils::log::warn("notify", &format!("系统通知发送失败: {}", e));
    }
}

fn write_secret_file(path: &std::path::Path, content: &str) -> std::io::Result<()> {
    std::fs::write(path, content.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o600);
            let _ = std::fs::set_permissions(path, perms);
        }
    }
    Ok(())
}

/// 托盘导入：明文数据直接导入；.spb 需要口令，交给启动器弹口令输入。
#[tauri::command]
fn import_from_path(
    app: AppHandle,
    state: State<'_, VaultState>,
    path: String,
    password: Option<String>,
) -> ApiResponse<ImportResult> {
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => return ApiResponse::err_message(format!("读取文件失败: {}", e)),
    };
    let mut g = state.lock().unwrap();
    let result = if path.to_lowercase().ends_with(".spb") {
        g.import_encrypted(data, password.unwrap_or_default())
    } else {
        g.import_passwords(data)
    };
    if let Some(r) = &result.data {
        notify(
            &app,
            &format!(
                "导入 {} 条，跳过 {} 条重复",
                r.imported_count, r.skipped_count
            ),
        );
    }
    result
}

/// 托盘导出：内容由后端生成，前端只负责收集导出密码和保存路径。
#[tauri::command]
fn export_to_path(
    app: AppHandle,
    state: State<'_, VaultState>,
    path: String,
    export_password: String,
) -> ApiResponse<String> {
    let content = {
        let mut g = state.lock().unwrap();
        match g.export_encrypted(&export_password, None) {
            ApiResponse { success: true, data: Some(c), .. } => c,
            ApiResponse { error, .. } => {
                return ApiResponse::err_message(error.unwrap_or_else(|| "导出失败".into()))
            }
        }
    };
    match write_secret_file(std::path::Path::new(&path), &content) {
        Ok(_) => {
            notify(&app, "密码已导出");
            ApiResponse::ok(path)
        }
        Err(e) => ApiResponse::err_message(format!("写入文件失败: {}", e)),
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
    let _ = tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        tauri::WebviewUrl::App("setting.html".into()),
    )
    .title("SecurePass 设置")
    .inner_size(720.0, 600.0)
    .min_inner_size(640.0, 520.0)
    .background_color(tauri::window::Color(24, 24, 27, 255))
    .build();
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
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

            // 全局热键：Ctrl+Alt+Space
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
            if let Err(e) = app.global_shortcut().register(shortcut) {
                crate::utils::log::error(
                    "hotkey",
                    &format!("全局热键 Ctrl+Alt+Space 注册失败: {}", e),
                );
            }

            // 系统托盘菜单
            use tauri::menu::{MenuBuilder, MenuItem, PredefinedMenuItem};
            let search = MenuItem::with_id(app, "search", "搜索", true, None::<&str>)
                .expect("创建菜单项失败");
            let lock = MenuItem::with_id(app, "lock", "立即锁定", true, None::<&str>)
                .expect("创建菜单项失败");
            let import = MenuItem::with_id(app, "import", "导入密码…", true, None::<&str>)
                .expect("创建菜单项失败");
            let export = MenuItem::with_id(app, "export", "导出密码…", true, None::<&str>)
                .expect("创建菜单项失败");
            let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)
                .expect("创建菜单项失败");
            let quit = MenuItem::with_id(app, "quit", "退出 SecurePass", true, None::<&str>)
                .expect("创建菜单项失败");
            let sep1 = PredefinedMenuItem::separator(app).expect("创建分隔符失败");
            let sep2 = PredefinedMenuItem::separator(app).expect("创建分隔符失败");
            let menu = MenuBuilder::new(app)
                .items(&[&search, &lock, &sep1, &import, &export, &settings, &sep2, &quit])
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
                    // 导入/导出：原生文件框选路径，.spb 的口令交给启动器收集
                    "import" => {
                        let handle = app.clone();
                        app.dialog()
                            .file()
                            .set_title("选择要导入的文件")
                            .add_filter("SecurePass 导出包", &["spb"])
                            .add_filter("明文数据", &["json", "csv"])
                            .pick_file(move |picked| {
                                let Some(fp) = picked else { return };
                                let Ok(path) = fp.into_path() else { return };
                                let path_str = path.to_string_lossy().to_string();
                                if path_str.to_lowercase().ends_with(".spb") {
                                    show_launcher(&handle);
                                    let _ = handle.emit(
                                        "tray-prompt",
                                        serde_json::json!({ "kind": "import", "path": path_str }),
                                    );
                                } else {
                                    match std::fs::read_to_string(&path) {
                                        Ok(text) => {
                                            if let Some(state) = handle.try_state::<VaultState>() {
                                                let res = state.lock().unwrap().import_passwords(text);
                                                if let Some(r) = res.data {
                                                    notify(&handle, &format!(
                                                        "导入 {} 条，跳过 {} 条重复",
                                                        r.imported_count, r.skipped_count
                                                    ));
                                                } else if let Some(e) = res.error {
                                                    notify(&handle, &format!("导入失败: {}", e));
                                                }
                                            }
                                        }
                                        Err(e) => notify(&handle, &format!("读取文件失败: {}", e)),
                                    }
                                }
                            });
                    }
                    "export" => {
                        let handle = app.clone();
                        app.dialog()
                            .file()
                            .set_title("导出加密密码包")
                            .set_file_name("securepass-export.spb")
                            .add_filter("SecurePass 导出包", &["spb"])
                            .save_file(move |picked| {
                                let Some(fp) = picked else { return };
                                let Ok(path) = fp.into_path() else { return };
                                show_launcher(&handle);
                                let _ = handle.emit(
                                    "tray-prompt",
                                    serde_json::json!({
                                        "kind": "export",
                                        "path": path.to_string_lossy(),
                                    }),
                                );
                            });
                    }
                    "settings" => open_settings(app),
                    "quit" => app.exit(0),
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
            check_password_strength,
            get_password_policy,
            get_expiring_passwords,
            list_entry_summaries,
            vault_status,
            copy_secret,
            reveal_password,
            import_from_path,
            export_to_path,
            get_entry_detail,
            resize_launcher,
            hide_launcher,
            set_suppress_hide
        ])
        .run(tauri::generate_context!())
        .expect("运行 SecurePass 失败");
}
