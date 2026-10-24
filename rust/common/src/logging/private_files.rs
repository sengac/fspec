use std::path::Path;
use tracing_appender::rolling::{Builder, RollingFileAppender, Rotation};

pub fn appender(directory: &Path, prefix: &str) -> std::io::Result<RollingFileAppender> {
    std::fs::create_dir_all(directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))?;
    }
    Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix(prefix)
        .max_log_files(5)
        .build(directory)
        .map_err(std::io::Error::other)
}
