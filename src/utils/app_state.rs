use crate::database::Database;
use std::path::PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct AppState {
    pub database: Database,
    pub app_data_dir: PathBuf,
    pub session_start: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

impl AppState {
    pub fn new(database: Database, app_data_dir: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            database,
            app_data_dir,
            session_start: now,
            last_activity: now,
        }
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
    }

    pub fn get_session_duration(&self) -> chrono::Duration {
        self.last_activity - self.session_start
    }

    pub fn is_session_expired(&self, max_duration: chrono::Duration) -> bool {
        self.get_session_duration() > max_duration
    }
}