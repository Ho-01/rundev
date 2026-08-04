pub(super) fn is_developer_app(identifier: &str) -> bool {
    let identifier = identifier.to_ascii_lowercase();

    #[cfg(windows)]
    return WINDOWS_APPS.contains(&identifier.as_str());

    #[cfg(target_os = "macos")]
    return MACOS_APPS.contains(&identifier.as_str());

    #[cfg(not(any(windows, target_os = "macos")))]
    false
}

pub(super) fn is_rundev(identifier: &str) -> bool {
    matches!(
        identifier.to_ascii_lowercase().as_str(),
        "rundev.exe" | "dev.rundev.app"
    )
}

pub(crate) fn display_name(identifier: &str) -> String {
    match identifier.to_ascii_lowercase().as_str() {
        "code.exe" | "com.microsoft.vscode" => "VS Code",
        "code-insiders.exe" | "com.microsoft.vscodeinsiders" => "VS Code Insiders",
        "cursor.exe" | "com.todesktop.230313mzl4w4u92" | "com.todesktop.230313mzl4w4u92-setapp" => {
            "Cursor"
        }
        "windsurf.exe" => "Windsurf",
        "idea64.exe" | "com.jetbrains.intellij" => "IntelliJ IDEA",
        "webstorm64.exe" | "com.jetbrains.webstorm" => "WebStorm",
        "pycharm64.exe" | "com.jetbrains.pycharm" => "PyCharm",
        "clion64.exe" | "com.jetbrains.clion" => "CLion",
        "rider64.exe" | "com.jetbrains.rider" => "Rider",
        "goland64.exe" | "com.jetbrains.goland" => "GoLand",
        "datagrip64.exe" | "com.jetbrains.datagrip" => "DataGrip",
        "phpstorm64.exe" | "com.jetbrains.phpstorm" => "PhpStorm",
        "rustrover64.exe" | "com.jetbrains.rustrover" => "RustRover",
        "studio64.exe" | "com.google.android.studio" => "Android Studio",
        "devenv.exe" => "Visual Studio",
        "windowsterminal.exe" => "Windows Terminal",
        "powershell.exe" | "pwsh.exe" => "PowerShell",
        "cmd.exe" => "Command Prompt",
        "wezterm-gui.exe" | "com.github.wez.wezterm" => "WezTerm",
        "alacritty.exe" | "org.alacritty" => "Alacritty",
        "com.googlecode.iterm2" => "iTerm2",
        "com.mitchellh.ghostty" => "Ghostty",
        "com.warp.warp-stable" | "dev.warp.warp-stable" => "Warp",
        "com.apple.dt.xcode" => "Xcode",
        "zed.exe" | "dev.zed.zed" => "Zed",
        "docker desktop.exe" | "com.docker.docker" => "Docker Desktop",
        "postman.exe" | "com.postmanlabs.mac" => "Postman",
        "githubdesktop.exe" | "com.github.githubclient" => "GitHub Desktop",
        _ => identifier.trim_end_matches(".exe"),
    }
    .to_string()
}

#[cfg(windows)]
const WINDOWS_APPS: &[&str] = &[
    "alacritty.exe",
    "cmd.exe",
    "code-insiders.exe",
    "code.exe",
    "cursor.exe",
    "datagrip64.exe",
    "devenv.exe",
    "docker desktop.exe",
    "githubdesktop.exe",
    "goland64.exe",
    "idea64.exe",
    "phpstorm64.exe",
    "postman.exe",
    "powershell.exe",
    "pwsh.exe",
    "pycharm64.exe",
    "rider64.exe",
    "rustrover64.exe",
    "studio64.exe",
    "webstorm64.exe",
    "wezterm-gui.exe",
    "windowsterminal.exe",
    "windsurf.exe",
    "zed.exe",
];

#[cfg(target_os = "macos")]
const MACOS_APPS: &[&str] = &[
    "com.apple.dt.xcode",
    "com.docker.docker",
    "com.github.githubclient",
    "com.github.wez.wezterm",
    "com.google.android.studio",
    "com.googlecode.iterm2",
    "com.jetbrains.clion",
    "com.jetbrains.datagrip",
    "com.jetbrains.goland",
    "com.jetbrains.intellij",
    "com.jetbrains.phpstorm",
    "com.jetbrains.pycharm",
    "com.jetbrains.rider",
    "com.jetbrains.rustrover",
    "com.jetbrains.webstorm",
    "com.microsoft.vscode",
    "com.microsoft.vscodeinsiders",
    "com.mitchellh.ghostty",
    "com.postmanlabs.mac",
    "com.todesktop.230313mzl4w4u92",
    "com.todesktop.230313mzl4w4u92-setapp",
    "com.warp.warp-stable",
    "dev.warp.warp-stable",
    "dev.zed.zed",
    "org.alacritty",
];

#[cfg(test)]
mod tests {
    use super::{display_name, is_developer_app, is_rundev};

    #[test]
    fn matches_known_apps_case_insensitively() {
        #[cfg(windows)]
        {
            assert!(is_developer_app("Code.exe"));
            assert!(is_developer_app("WINDOWSTERMINAL.EXE"));
        }

        #[cfg(target_os = "macos")]
        {
            assert!(is_developer_app("com.microsoft.VSCode"));
            assert!(is_developer_app("com.apple.dt.Xcode"));
        }
    }

    #[test]
    fn rejects_partial_or_unrelated_names() {
        assert!(!is_developer_app("my-code-wrapper.exe"));
        assert!(!is_developer_app("com.apple.Safari"));
    }

    #[test]
    fn maps_platform_identifiers_to_the_same_display_name() {
        assert_eq!(display_name("code.exe"), "VS Code");
        assert_eq!(display_name("com.microsoft.VSCode"), "VS Code");
        assert_eq!(display_name("cursor.exe"), "Cursor");
    }

    #[test]
    fn identifies_rundev_on_supported_platforms() {
        assert!(is_rundev("RunDev.exe"));
        assert!(is_rundev("dev.rundev.app"));
        assert!(!is_rundev("com.microsoft.VSCode"));
    }
}
