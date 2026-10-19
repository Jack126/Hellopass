use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use chrono::serde::{ts_seconds, ts_seconds_option};
use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    Manager, Result, State
};

// 数据库状态
#[derive(Debug, Clone)]
pub struct DatabaseState {
    pub db: SimpleDatabase,
}

impl DatabaseState {
    pub fn new() -> Self {
        Self {
            db: SimpleDatabase::new(),
        }
    }
}

// 简化的密码条目结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordEntry {
    pub id: i64,
    pub site_name: String,
    pub username: String,
    pub encrypted_password: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPassword {
    pub site_name: String,
    pub username: String,
    pub password: String,
}

// 简化的数据库操作
pub struct SimpleDatabase {
    pub entries: Vec<PasswordEntry>,
}

impl SimpleDatabase {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add_password(&mut self, password: NewPassword) -> PasswordEntry {
        let entry = PasswordEntry {
            id: self.entries.len() as i64 + 1,
            site_name: password.site_name,
            username: password.username,
            encrypted_password: password.password,
            created_at: Utc::now().to_string(),
        };
        self.entries.push(entry.clone());
        entry
    }

    pub fn get_passwords(&self) -> Vec<PasswordEntry> {
        self.entries.clone()
    }

    pub fn delete_password(&mut self, id: i64) -> bool {
        let index = self.entries.iter().position(|p| p.id == id);
        if let Some(index) = index {
            self.entries.remove(index);
            true
        } else {
            false
        }
    }
}

// API响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    #[serde(with = "ts_seconds")]
    pub timestamp: DateTime<Utc>,
}

// Tauri命令
#[tauri::command]
async fn add_password(
    state: State<DatabaseState>,
    password: NewPassword,
) -> Result<ApiResponse<PasswordEntry>> {
    let mut db = state.db.clone();
    let entry = db.add_password(password);
    Ok(ApiResponse {
        success: true,
        data: Some(entry),
        error: None,
        timestamp: Utc::now(),
    })
}

#[tauri::command]
async fn get_passwords(state: State<DatabaseState>) -> Result<ApiResponse<Vec<PasswordEntry>>> {
    let passwords = state.db.get_passwords();
    Ok(ApiResponse {
        success: true,
        data: Some(passwords),
        error: None,
        timestamp: Utc::now(),
    })
}

#[tauri::command]
async fn delete_password(
    state: State<DatabaseState>,
    id: i64,
) -> Result<ApiResponse<()>> {
    let mut db = state.db.clone();
    let success = db.delete_password(id);
    Ok(ApiResponse {
        success,
        data: Some(()),
        error: if success { None } else { Some("Password not found".to_string()) },
        timestamp: Utc::now(),
    })
}

#[tauri::command]
async fn show_notification(message: String) {
    println!("Notification: {}", message);
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // 创建数据库状态
            let db_state = DatabaseState::new();
            app.manage(db_state);

            // 创建菜单
            let quit = MenuItem::with_id(app.handle(), "quit", "Quit", true, None::<&str>)?;
            let menu = MenuBuilder::new(app.handle())
                .item(&quit)
                .build()?;

            // 创建系统托盘
            let _tray = TrayIconBuilder::with_id("main-tray")
                .menu(&menu)
                .tooltip("SecurePass")
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            position: _,
                            size: _,
                        } => {
                            if let Some(window) = tray.get_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        TrayIconEvent::Click {
                            button: MouseButton::Right,
                            position: _,
                            size: _,
                        } => {
                            // 显示右键菜单
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // 创建主窗口
            let window = tauri::WindowBuilder::new(app, "main", tauri::WindowUrl::App("index.html".into()))
                .title("SecurePass - Password Manager")
                .inner_size(900.0, 700.0)
                .min_inner_size(600.0, 500.0)
                .build()?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            add_password,
            get_passwords,
            delete_password,
            show_notification,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}