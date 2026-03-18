fn main() {
    // Embed icon and version info into the Windows exe
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "ClaudeTerm");
        res.set("FileDescription", "Themed terminal for Claude Code on Windows");
        res.set("LegalCopyright", "MIT License");
        res.compile().expect("Failed to compile Windows resources");
    }
}
