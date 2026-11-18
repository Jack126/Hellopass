use std::path::Path;

/// 写入文件并收紧为仅属主可读写（unix 0600）。
/// Windows 上沿用默认 ACL（用户配置目录本身已按用户隔离）。
pub fn write_private(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    let path = path.as_ref();
    std::fs::write(path, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}
