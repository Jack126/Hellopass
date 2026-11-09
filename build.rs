#[cfg(feature = "tauri-app")]
fn main() {
    // tauri_build 读取 tauri.conf.json 并为 generate_context!() 生成上下文。
    tauri_build::build();
}

#[cfg(not(feature = "tauri-app"))]
fn main() {}
