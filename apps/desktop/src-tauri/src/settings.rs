//! Client folder selection only. Never persists permission grants or active runs.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LastFolder {
    version: u32,
    path: PathBuf,
}
pub fn load(directory: &Path) -> io::Result<Option<PathBuf>> {
    let mut file = match fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(directory.join("desktop-folder.json"))
    {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    if !file.metadata()?.is_file() || file.metadata()?.len() > 8192 {
        return Err(io::Error::other("invalid folder selection file"));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file).take(8193).read_to_end(&mut bytes)?;
    let record: LastFolder = serde_json::from_slice(&bytes)
        .map_err(|_| io::Error::other("corrupt desktop folder selection"))?;
    if record.version != 1 {
        return Err(io::Error::other("unsupported desktop folder selection"));
    }
    Ok(Some(record.path))
}
pub fn save(directory: &Path, path: &Path) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    if fs::symlink_metadata(directory)?.file_type().is_symlink() {
        return Err(io::Error::other("symlinked configuration directory"));
    }
    fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
    let target = directory.join("desktop-folder.json");
    if fs::symlink_metadata(&target).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(io::Error::other("symlinked folder selection"));
    }
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    file.as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))?;
    serde_json::to_writer(
        &mut file,
        &LastFolder {
            version: 1,
            path: path.into(),
        },
    )
    .map_err(io::Error::other)?;
    file.write_all(b"\n")?;
    file.as_file().sync_all()?;
    file.persist(target).map_err(|e| e.error)?;
    fs::File::open(directory)?.sync_all()
}
