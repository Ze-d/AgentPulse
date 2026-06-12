use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    prepare_hook_resources();
    tauri_build::build()
}

fn prepare_hook_resources() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("src-tauri is nested under apps/desktop");
    let hook_dir = repo_root.join("adapters").join("hook-adapter");

    println!(
        "cargo:rerun-if-changed={}",
        hook_dir.join("Cargo.toml").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        hook_dir.join("Cargo.lock").display()
    );
    println!("cargo:rerun-if-changed={}", hook_dir.join("src").display());

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let target = env::var("TARGET").ok();
    let mut command = Command::new(cargo);
    command
        .arg("build")
        .arg("--release")
        .arg("--locked")
        .arg("--manifest-path")
        .arg(hook_dir.join("Cargo.toml"));

    if let Some(target) = target.as_deref() {
        command.arg("--target").arg(target);
    }

    let status = command
        .status()
        .expect("failed to invoke cargo for hook-adapter build");
    if !status.success() {
        panic!("failed to build hook-adapter release binary");
    }

    let bin_name = if target.as_deref().is_some_and(|t| t.contains("windows")) {
        "agentpulse-hook.exe"
    } else {
        "agentpulse-hook"
    };

    let mut built_binary = hook_dir.join("target");
    if let Some(target) = target.as_deref() {
        built_binary = built_binary.join(target);
    }
    built_binary = built_binary.join("release").join(bin_name);

    let resources_dir = manifest_dir.join("resources");
    fs::create_dir_all(&resources_dir).expect("failed to create generated resource directory");

    for resource_name in ["agentpulse-hook", "agentpulse-hook.exe"] {
        fs::copy(&built_binary, resources_dir.join(resource_name))
            .unwrap_or_else(|err| panic!("failed to copy hook resource {resource_name}: {err}"));
    }
}
