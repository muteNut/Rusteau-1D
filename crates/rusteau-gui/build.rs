//! Windows: the program icon (assets/icon.ico) is embedded in rusteau.exe, so the Explorer and the task bar show it.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            // without the resource compiler the program still builds, without the icon in the Explorer
            println!("cargo:warning=the icon could not be embedded: {e}");
        }
    }
}
