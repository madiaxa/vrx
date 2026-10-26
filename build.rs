//! Embed the supplied Explorer icon on native MSVC builds using the Windows SDK.
use std::{io::Write, path::PathBuf};
fn build() -> Result<(), Box<dyn std::error::Error>> {
    let mut out = std::io::stdout().lock();
    writeln!(out, "cargo:rerun-if-changed=assets/icons/VRX.ico")?;
    writeln!(out, "cargo:rerun-if-changed=build.rs")?;
    let target = std::env::var("TARGET")?;
    let host = std::env::var("HOST")?;
    if !target.contains("windows-msvc") || !host.contains("windows") {
        return Ok(());
    }
    let directory =
        PathBuf::from(std::env::var_os("OUT_DIR").ok_or("Missing Cargo output directory")?);
    let icon = PathBuf::from("assets/icons/VRX.ico").canonicalize()?;
    let resource = directory.join("vrx.rc");
    let compiled = directory.join("vrx.res");
    std::fs::write(
        &resource,
        format!(
            "1 ICON \"{}\"\n",
            icon.display().to_string().replace('\\', "/")
        ),
    )?;
    let mut candidates = vec![PathBuf::from("rc.exe")];
    if let Some(root) = std::env::var_os("ProgramFiles(x86)") {
        let sdk = PathBuf::from(root).join("Windows Kits/10/bin");
        if let Ok(entries) = std::fs::read_dir(sdk) {
            let mut paths: Vec<_> = entries
                .filter_map(Result::ok)
                .map(|e| e.path().join("x64/rc.exe"))
                .filter(|p| p.is_file())
                .collect();
            paths.sort();
            paths.reverse();
            candidates.extend(paths);
        }
    }
    let mut success = false;
    for compiler in candidates {
        if let Ok(status) = std::process::Command::new(compiler)
            .arg("/nologo")
            .arg("/fo")
            .arg(&compiled)
            .arg(&resource)
            .status()
            && status.success()
        {
            success = true;
            break;
        }
    }
    if !success {
        return Err("Windows SDK rc.exe is required to embed VRX's icon".into());
    }
    writeln!(out, "cargo:rustc-link-arg-bin=vrx={}", compiled.display())?;
    Ok(())
}
fn main() {
    if let Err(error) = build() {
        let _ = writeln!(std::io::stderr().lock(), "VRX build: {error}");
        std::process::exit(1);
    }
}
