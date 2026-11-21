//! On Windows the program file gets Loot Gobbler's icon (Explorer, the taskbar before it starts) and its version.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon/lootgobbler.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icon/lootgobbler.ico");
        res.set("ProductName", "Loot Gobbler");
        res.set("FileDescription", "Loot Gobbler");
        if let Err(e) = res.compile() {
            println!("cargo:warning=no icon for the program file: {e}");
        }
    }
}
