use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug)]
pub struct Toolchain {
    pub clang: PathBuf,
    pub lld: PathBuf,
    pub clang_version: String,
    pub lld_version: String,
}

pub fn run<I, S>(program: &Path, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let result = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Run {}", program.display()))?;
    ensure!(
        result.status.success(),
        "Tool failed: {}\n{}",
        program.display(),
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(())
}

fn resolve(command: &str) -> Result<(PathBuf, String)> {
    let input = Path::new(command);
    let candidates: Vec<PathBuf> = if input.components().count() > 1 || input.is_absolute() {
        vec![input.to_owned()]
    } else {
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|dir| dir.join(input))
            .collect()
    };
    for candidate in candidates {
        if !candidate.is_file() {
            continue;
        }
        // Multicall tools such as LLD select their driver from argv[0]. Keep the
        // invoked symlink name rather than resolving ld.lld to a generic lld.
        let path = if candidate.is_absolute() {
            candidate
        } else {
            std::env::current_dir()?.join(candidate)
        };
        let result = match Command::new(&path).arg("--version").output() {
            Ok(value) => value,
            Err(_) => continue,
        };
        ensure!(
            result.status.success(),
            "Tool failed: {} --version",
            path.display()
        );
        let version = String::from_utf8(result.stdout)?
            .lines()
            .next()
            .context("Tool returned no version")?
            .to_owned();
        return Ok((path, version));
    }
    bail!("Required tool not found: {command}; provide its executable path")
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn resolution_preserves_multicall_symlink_name() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let driver = directory.path().join("driver");
        std::fs::write(&driver, "#!/bin/sh\nprintf '%s\\n' \"$0\"\n")?;
        std::fs::set_permissions(&driver, std::fs::Permissions::from_mode(0o755))?;
        let alias = directory.path().join("ld.lld");
        symlink(&driver, &alias)?;
        let (path, version) = resolve(alias.to_str().unwrap())?;
        assert_eq!(path, alias);
        assert_eq!(version, alias.to_str().unwrap());
        Ok(())
    }
}

impl Toolchain {
    pub fn discover(clang: &str, lld: &str) -> Result<Self> {
        let (clang, clang_version) = resolve(clang)?;
        let (lld, lld_version) = resolve(lld)?;
        let tools = Self {
            clang,
            lld,
            clang_version,
            lld_version,
        };
        tools.probe()?;
        Ok(tools)
    }
    pub fn manifest(&self) -> Value {
        json!({"clang": self.clang_version, "lld": self.lld_version})
    }
    pub fn probe(&self) -> Result<()> {
        let dir = tempfile::Builder::new()
            .prefix("grkit-toolchain-")
            .tempdir()?;
        let source = dir.path().join("probe.c");
        std::fs::write(&source, "void _start(void) { for (;;) {} }\n")?;
        for target in ["arm-none-eabi", "armv7-linux-gnueabi"] {
            let object = dir.path().join(format!("{target}.o"));
            run(
                &self.clang,
                [
                    OsStr::new(&format!("--target={target}")),
                    OsStr::new("-march=armv7-a"),
                    OsStr::new("-marm"),
                    OsStr::new("-ffreestanding"),
                    OsStr::new("-fno-builtin"),
                    OsStr::new("-c"),
                    source.as_os_str(),
                    OsStr::new("-o"),
                    object.as_os_str(),
                ],
            )?;
        }
        run(
            &self.lld,
            [
                OsStr::new("-m"),
                OsStr::new("armelf_linux_eabi"),
                OsStr::new("-static"),
                OsStr::new("-e"),
                OsStr::new("_start"),
                OsStr::new("--build-id=none"),
                OsStr::new("-o"),
                dir.path().join("probe").as_os_str(),
                dir.path().join("armv7-linux-gnueabi.o").as_os_str(),
            ],
        )
    }
}
