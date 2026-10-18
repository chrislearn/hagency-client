fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args_os().nth(1).unwrap_or_else(|| ".".into());
    let packed = rinx_system_apps::pack(std::path::Path::new(&root))?;
    for app in packed.apps {
        println!(
            "{} {} {:?} {}",
            app.manifest.id, app.manifest.version, app.native, app.manifest.integrity.bundle_blake3
        );
    }
    Ok(())
}
