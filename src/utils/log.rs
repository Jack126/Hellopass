#![allow(dead_code)]

use chrono::Utc;
use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn label(self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

pub fn emit(level: LogLevel, module: &str, message: &str) {
    let ts = Utc::now().to_rfc3339();
    let _ = writeln!(std::io::stderr(), "[{}][{}][{}] {}", ts, level.label(), module, message);
}

pub fn debug(module: &str, message: &str) { emit(LogLevel::Debug, module, message); }
pub fn info(module: &str, message: &str) { emit(LogLevel::Info, module, message); }
pub fn warn(module: &str, message: &str) { emit(LogLevel::Warn, module, message); }
pub fn error(module: &str, message: &str) { emit(LogLevel::Error, module, message); }
