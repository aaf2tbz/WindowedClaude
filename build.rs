fn main() {
    // Embed icon and version info into the Windows exe
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();

        // Only set icon if the file exists (avoids CI failure before icon is designed)
        if std::path::Path::new("assets/icon.ico").exists() {
            res.set_icon("assets/icon.ico");
        }

        res.set("ProductName", "WindowedClaude");
        res.set("FileDescription", "Themed terminal for Claude Code on Windows");
        res.set("LegalCopyright", "MIT License");

        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {e}");
            eprintln!("The exe will work but won't have a custom icon.");
        }
    }
}
