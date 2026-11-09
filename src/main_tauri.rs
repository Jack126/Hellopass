mod security;
mod database;
mod api;
mod utils;
mod error;

use std::sync::Mutex;
use tauri::State;
use tauri::Manager;
use crate::api::runtime::VaultRuntime;
use crate::api::dto::*;
use crate::security::policies::PasswordStrengthResult;

type VaultState = Mutex<VaultRuntime>;

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
fn export_passwords(state: State<'_, VaultState>) -> ApiResponse<String> {
    let mut g = state.lock().unwrap();
    g.export_passwords()
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
fn export_encrypted(state: State<'_, VaultState>, export_password: String) -> ApiResponse<String> {
    let mut g = state.lock().unwrap();
    g.export_encrypted(&export_password)
}

#[tauri::command]
fn import_encrypted(state: State<'_, VaultState>, data: String, export_password: String) -> ApiResponse<ImportResult> {
    let mut g = state.lock().unwrap();
    g.import_encrypted(data, export_password)
}

#[tauri::command]
fn export_passwords_csv(state: State<'_, VaultState>) -> ApiResponse<String> {
    let mut g = state.lock().unwrap();
    g.export_passwords_csv()
}

#[tauri::command]
fn export_selected(state: State<'_, VaultState>, ids: Vec<i64>, format: String) -> ApiResponse<String> {
    let mut g = state.lock().unwrap();
    g.export_selected(ids, format)
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

            // 主窗口
            let _window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("SecurePass - 密码管理器")
            .inner_size(900.0, 700.0)
            .min_inner_size(600.0, 500.0)
            // 设不透明背景，避免某些 Linux/无合成器环境下 WebView 渲染为黑方块
            .background_color(tauri::window::Color(255, 255, 255, 255))
            .build()
            .expect("创建主窗口失败");

            // 系统托盘
            let quit = tauri::menu::MenuItem::with_id(
                app,
                "quit",
                "退出 SecurePass",
                true,
                None::<&str>,
            )
            .expect("创建菜单项失败");
            let menu = tauri::menu::MenuBuilder::new(app)
                .items(&[&quit])
                .build()
                .expect("创建菜单失败");

            let _tray = tauri::tray::TrayIconBuilder::with_id("securepass-tray")
                .icon(
                    app.default_window_icon()
                        .expect("缺少默认窗口图标")
                        .clone(),
                )
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id().as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
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
            export_passwords,
            export_passwords_csv,
            export_selected,
            import_passwords,
            export_backup,
            restore_backup,
            export_encrypted,
            import_encrypted,
            get_audit_logs,
            purge_old_audit_logs,
            check_integrity,
            check_password_strength,
            get_password_policy,
            get_expiring_passwords
        ])
        .run(tauri::generate_context!())
        .expect("运行 SecurePass 失败");
}
