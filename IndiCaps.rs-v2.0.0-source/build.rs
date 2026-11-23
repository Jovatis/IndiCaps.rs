use std::{env, fs, path::PathBuf, process::Command};

/// Embeds the official ICO as the executable icon without any Rust crate.
/// MSVC's resource compiler is part of the Windows SDK already required to
/// build this project, so this keeps the zero-dependency constraint intact.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/IndiCaps.ico");

    // GNU targets keep building, just without a custom PE icon.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
    {
        return;
    }

    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let ico = manifest.join("assets").join("IndiCaps.ico");
    assert!(ico.is_file(), "missing assets/IndiCaps.ico");

    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let rc = out.join("IndiCaps.rc");
    let res = out.join("IndiCaps.res");

    // rc.exe string literals treat backslashes as escapes.
    let ico_path = ico.to_string_lossy().replace('\\', "\\\\");
    fs::write(&rc, format!("1 ICON \"{}\"\n", ico_path)).unwrap();

    let rc_exe = find_rc().unwrap_or_else(|| PathBuf::from("rc.exe"));
    let status = Command::new(&rc_exe)
        .arg("/nologo")
        .arg("/fo")
        .arg(&res)
        .arg(&rc)
        .status()
        .unwrap_or_else(|error| panic!("failed to run rc.exe: {error}"));
    assert!(status.success(), "rc.exe failed to compile assets/IndiCaps.ico");

    println!("cargo:rustc-link-arg={}", res.display());
}

fn find_rc() -> Option<PathBuf> {
    if let Some(path) = env::var_os("RC").or_else(|| env::var_os("RC_EXE")) {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    let base = env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"));
    let bin = base.join("Windows Kits").join("10").join("bin");
    let mut versions: Vec<PathBuf> = fs::read_dir(&bin)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    versions.sort();

    for version in versions.iter().rev() {
        for arch in ["x64", "x86"] {
            let candidate = version.join(arch).join("rc.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
