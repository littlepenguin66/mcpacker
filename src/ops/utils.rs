use std::path::Path;

#[cfg(unix)]
/// Make file executable
pub fn make_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = std::fs::metadata(path)?;
    let mut perms = metadata.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
/// Make file executable
pub fn make_executable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}
