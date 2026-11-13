//! Generic secret storage in the login Keychain via `/usr/bin/security`.
//! Secrets are written through `security -i` on stdin, never in arguments.
use std::io::Write;

const ACCOUNT: &str = "astrid";

fn valid(secret: &str) -> bool {
    !secret.is_empty()
        && secret.len() < 4096
        && secret
            .chars()
            .all(|c| c.is_ascii_graphic() && c != '"' && c != '\\')
}

pub fn store(service: &str, label: &str, secret: &str) -> Result<(), String> {
    if !valid(secret)
        || !service
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.')
        || label.contains('"')
    {
        return Err("unexpected secret format".into());
    }
    let mut child = std::process::Command::new("/usr/bin/security")
        .arg("-i")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| "Keychain unavailable")?;
    let script = format!(
        "add-generic-password -U -s {service} -a {ACCOUNT} -l \"{label}\" -w \"{secret}\"\n"
    );
    child
        .stdin
        .take()
        .ok_or("Keychain unavailable")?
        .write_all(script.as_bytes())
        .map_err(|_| "Keychain write failed")?;
    if child.wait().map_err(|_| "Keychain write failed")?.success() {
        Ok(())
    } else {
        Err("Keychain write failed".into())
    }
}

pub fn read(service: &str) -> Result<Option<String>, String> {
    let output = std::process::Command::new("/usr/bin/security")
        .args(["find-generic-password", "-s", service, "-a", ACCOUNT, "-w"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|_| "Keychain unavailable")?;
    Ok(output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|s| valid(s)))
}

pub fn delete(service: &str) -> bool {
    std::process::Command::new("/usr/bin/security")
        .args(["delete-generic-password", "-s", service, "-a", ACCOUNT])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}
