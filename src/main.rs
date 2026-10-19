#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, clippy::cargo)]
#![warn(clippy::pedantic)]

use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, SystemTray};
use tracing::{info, error};

mod security;
mod database;
mod api;
mod utils;

use api::commands::*;
use utils::app_state::AppState;

fn main() {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    info!("Starting SecurePass application");

    // 创建系统托盘菜单
    let show_item = MenuItem::new(tauri::generate_id(), "Show", true, None);
    let hide_item = MenuItem::new(tauri::generate_id(), "Hide", true, None);
    let quit_item = MenuItem::new(tauri::generate_id(), "Quit", true, None);

    let menu = MenuBuilder::new()
        .item(&show_item)
        .item(&hide_item)
        .separator()
        .item(&quit_item)
        .build();

    let system_tray = SystemTray::new()
        .with_menu(menu)
        .with_icon(tauri::image::Image::from_bytes(include_bytes!("../assets/icon.png")).unwrap());

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // 获取应用目录
            let app_data_dir = app.path().app_data_dir().unwrap();
            std::fs::create_dir_all(&app_data_dir).unwrap();
            
            // 初始化数据库
            let database_path = app_data_dir.join("passwords.db");
            let database = database::Database::new(&database_path).expect("Failed to initialize database");
            
            // 初始化应用状态
            let app_state = AppState::new(database, app_data_dir);
            
            // 将应用状态存入应用数据
            app.manage(app_state.clone());
            
            // 系统托盘事件处理
            let app_handle = app.handle();
            
            app.tray_handle().on_menu_event(move |tray_event| {
                match tray_event.menu_item_id() {
                    id if id == show_item.id() => {
                        if let Some(window) = app_handle.get_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    id if id == hide_item.id() => {
                        if let Some(window) = app_handle.get_window("main") {
                            let _ = window.hide();
                        }
                    }
                    id if id == quit_item.id() => {
                        app_handle.exit(0);
                    }
                    _ => {}
                }
            });

            // 系统托盘图标点击事件
            app.tray_handle().on_tray_icon_event(|tray, event| {
                match event {
                    TrayIconEvent::Click { button, button_state, .. } => {
                        if button == MouseButton::Left && button_state == MouseButtonState::Up {
                            if let Some(window) = app_handle.get_window("main") {
                                let _ = match window.is_visible() {
                                    true => window.hide(),
                                    false => {
                                        window.show()?;
                                        window.set_focus()
                                    }
                                };
                            }
                        }
                    }
                    _ => {}
                }
            });

            // 设置主窗口
            let main_window = app.get_window("main").unwrap();
            
            // 设置窗口事件处理
            main_window.on_menu_event(move |event| {
                match event.menu_item_id() {
                    // 可以添加更多菜单项处理
                    _ => {}
                }
            });

            info!("Application setup completed");
            Ok(())
        })
        .system_tray(system_tray)
        .invoke_handler(tauri::generate_handler![
            // 认证相关
            login,
            logout,
            check_master_password,
            
            // 密码管理
            get_passwords,
            search_passwords,
            add_password,
            update_password,
            delete_password,
            copy_password,
            
            // 导入导出
            import_passwords,
            export_passwords,
            generate_export_file,
            import_from_file,
            
            // 密码生成
            generate_password,
            calculate_password_strength,
            
            // 配置管理
            get_config,
            update_config,
            
            // 安全相关
            verify_integrity,
            create_backup,
            restore_backup,
            
            // 系统相关
            get_system_info,
            check_updates,
            
            // 主密码管理
            set_master_password,
            change_master_password,
            verify_master_password,
            export_master_key,
            import_master_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}