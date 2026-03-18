fn main() {
    // Embed icon, version info, and application manifest into the Windows exe
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();

        // Only set icon if the file exists (avoids CI failure before icon is designed)
        if std::path::Path::new("assets/icon.ico").exists() {
            res.set_icon("assets/icon.ico");
        }

        // PE version info — SmartScreen uses these heuristics for reputation
        res.set("ProductName", "WindowedClaude");
        res.set("FileDescription", "WindowedClaude — Themed Terminal for Claude Code");
        res.set("CompanyName", "WindowedClaude");
        res.set("InternalName", "windowed-claude");
        res.set("OriginalFilename", "windowed-claude.exe");
        res.set("LegalCopyright", "Copyright (c) 2026 WindowedClaude. MIT License.");
        res.set_version_info(winresource::VersionInfo::PRODUCTVERSION, 0x0001_0000_0003_0000); // 1.0.3.0
        res.set_version_info(winresource::VersionInfo::FILEVERSION, 0x0001_0000_0003_0000);

        // Embed the application manifest for UAC, DPI awareness, and OS compatibility
        if std::path::Path::new("assets/app.manifest").exists() {
            res.set_manifest_file("assets/app.manifest");
        }

        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {e}");
            eprintln!("The exe will work but won't have a custom icon.");
        }
    }
}
