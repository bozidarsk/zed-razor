// Adapted from two sources:
//  - Download/launch mechanism: the official Zed C# extension's
//    src/language_servers/roslyn.rs (https://github.com/zed-extensions/csharp).
//    Downloads the self-contained `roslyn-language-server` build straight
//    from the Microsoft NuGet feed, so no separate `dotnet tool install`
//    step (or even a .NET SDK) is required for the baseline path.
//  - Razor cohosting additions: the `feature/razor-and-more` branch of
//    https://github.com/kevin-mueller/zed-csharp
//    (src/language_servers/roslyn_official.rs) — the
//    `razor|language_server.cohosting_enabled` setting and the optional
//    "build Microsoft's own VS Code Razor extension from source" bootstrap.

use std::fs;

use zed_extension_api::{self as zed, LanguageServerId, Result, settings::LspSettings};

use super::{nuget::NuGetClient, util};

const PACKAGE_PREFIX: &str = "roslyn-language-server";
const SERVER_BINARY: &str = "Microsoft.CodeAnalysis.LanguageServer";

pub struct Roslyn {
    cached_server_path: Option<ServerPath>,
    nuget: NuGetClient,
}

impl Roslyn {
    // Distinct from the official C# extension's own "roslyn" id, so this
    // gets its own process/log/settings ("lsp.roslyn-razor" in Zed
    // settings.json) even if that extension is also installed.
    pub const LANGUAGE_SERVER_ID: &'static str = "roslyn-razor";

    pub fn new() -> Self {
        Roslyn {
            cached_server_path: None,
            nuget: NuGetClient::new(),
        }
    }

    pub fn language_server_cmd(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let binary_settings = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.binary);
        let user_args = binary_settings
            .as_ref()
            .and_then(|binary_settings| binary_settings.arguments.clone());

        let mut extra_args = vec!["--stdio".to_string(), "--autoLoadProjects".to_string()];
        if let Some(razor_args) = Self::install_or_update_razor(worktree, language_server_id)? {
            extra_args.extend(razor_args);
        }
        if let Some(args) = user_args {
            extra_args.extend(args);
        }

        // Manual override: point straight at a user-managed installation.
        if let Some(path) = binary_settings.and_then(|binary_settings| binary_settings.path) {
            return Ok(zed::Command {
                command: path,
                args: extra_args,
                env: Default::default(),
            });
        }

        if let Some(ref server_path) = self.cached_server_path {
            if fs::metadata(server_path.as_str()).is_ok_and(|stat| stat.is_file()) {
                return Ok(Self::build_command(server_path, extra_args));
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let rid = match zed::current_platform() {
            (zed::Os::Windows, zed::Architecture::X8664) => "win-x64",
            (zed::Os::Windows, zed::Architecture::Aarch64) => "win-arm64",
            (zed::Os::Linux, zed::Architecture::X8664) => "linux-x64",
            (zed::Os::Linux, zed::Architecture::Aarch64) => "linux-arm64",
            (zed::Os::Mac, zed::Architecture::X8664) => "osx-x64",
            (zed::Os::Mac, zed::Architecture::Aarch64) => "osx-arm64",
            _ => "any",
        };

        let package_id = format!("{PACKAGE_PREFIX}.{rid}");
        let version = self.nuget.get_latest_version(&package_id)?;
        let version_dir = format!("{}-{}", Self::LANGUAGE_SERVER_ID, version);

        let already_installed = Self::find_server_path(rid, &version_dir)
            .is_ok_and(|sp| fs::metadata(sp.as_str()).is_ok_and(|stat| stat.is_file()));

        if !already_installed {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            self.nuget
                .download_and_extract(&package_id, &version, &version_dir)?;

            util::remove_outdated_versions(Self::LANGUAGE_SERVER_ID, &version_dir)?;
        }

        let server_path = Self::find_server_path(rid, &version_dir)?;
        if let ServerPath::Exe(ref path) = server_path {
            zed::make_file_executable(path)?;
        }

        let command = Self::build_command(&server_path, extra_args);
        self.cached_server_path = Some(server_path);
        Ok(command)
    }

    fn build_command(server_path: &ServerPath, extra_args: Vec<String>) -> zed::Command {
        match server_path {
            ServerPath::Dll(path) => {
                let mut args = vec!["exec".to_string(), path.clone()];
                args.extend(extra_args);
                zed::Command {
                    command: "dotnet".to_string(),
                    args,
                    env: Default::default(),
                }
            }
            ServerPath::Exe(path) => zed::Command {
                command: path.clone(),
                args: extra_args,
                env: Default::default(),
            },
        }
    }

    fn find_server_path(rid: &str, version_dir: &str) -> Result<ServerPath> {
        let tools_dir = format!("{version_dir}/tools");

        let tfm = fs::read_dir(&tools_dir)
            .map_err(|e| format!("failed to read tools directory '{tools_dir}': {e}"))?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                if entry.file_type().ok()?.is_dir() {
                    entry.file_name().into_string().ok()
                } else {
                    None
                }
            })
            .next()
            .ok_or_else(|| format!("no TFM directory found inside '{tools_dir}'"))?;

        let server_dir = format!("{tools_dir}/{tfm}/{rid}");
        match Self::server_path_for_rid(rid, server_dir) {
            ServerPath::Dll(path) => Ok(ServerPath::Dll(util::absolute_path(&path)?)),
            exe => Ok(exe),
        }
    }

    fn server_path_for_rid(rid: &str, server_dir: String) -> ServerPath {
        if rid == "any" {
            ServerPath::Dll(format!("{server_dir}/{SERVER_BINARY}.dll"))
        } else if rid.starts_with("win-") {
            ServerPath::Exe(format!("{server_dir}/{SERVER_BINARY}.exe"))
        } else {
            ServerPath::Exe(format!("{server_dir}/{SERVER_BINARY}"))
        }
    }

    pub fn configuration_options(
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let settings = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings);

        // Unlike the official extension, we send this unconditionally (not
        // only when the user already has an `lsp.roslyn-razor.settings`
        // block) so cohosting is on by default the moment this server runs.
        Ok(Some(Self::transform_settings_for_roslyn(
            settings.unwrap_or(zed::serde_json::Value::Null),
        )))
    }

    fn transform_settings_for_roslyn(settings: zed::serde_json::Value) -> zed::serde_json::Value {
        let mut roslyn_config = zed::serde_json::json!({
            // The actual switch that turns on Roslyn's in-process Razor
            // support (completions/hover/etc. for @code, @expressions and
            // Tag Helpers). Everything else here is optional quality-of-life,
            // mirrored from the official C# extension's defaults.
            "razor|language_server.cohosting_enabled": true,
            "csharp|code_lens.dotnet_enable_references_code_lens": false,
            "csharp|code_lens.dotnet_enable_tests_code_lens": false,
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_parameters": true,
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_literal_parameters": true,
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_indexer_parameters": true,
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_object_creation_parameters": true,
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_other_parameters": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_types": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_implicit_variable_types": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_lambda_parameter_types": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_implicit_object_creation": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_collection_expressions": true,
        });

        let config_map = roslyn_config.as_object_mut().unwrap();
        if let zed::serde_json::Value::Object(settings_map) = settings {
            for (key, value) in settings_map {
                if key.contains('|') {
                    if let zed::serde_json::Value::Object(nested_settings) = value {
                        for (nested_key, nested_value) in nested_settings {
                            config_map.insert(format!("{key}.{nested_key}"), nested_value);
                        }
                    }
                } else if key.contains('.') {
                    config_map.insert(key.clone(), value.clone());
                }
            }
        }

        roslyn_config
    }

    /// Optional, opt-in: if the user sets `roslyn_source_repository_root` in
    /// their `lsp.roslyn-razor.settings`, clones/updates dotnet/roslyn at
    /// that path, builds Microsoft's own VS Code Razor extension from it,
    /// and returns the extra `--extension`/`--razorSourceGenerator`/
    /// `--razorDesignTimePath` args Roslyn needs to load it.
    ///
    /// Try WITHOUT this first (just `cohosting_enabled: true` above) — the
    /// NuGet-published roslyn-language-server may already bundle enough
    /// Razor cohosting support on its own. Reach for this only if @-context
    /// completions/Tag Helpers still aren't showing up.
    ///
    /// Requires `git` and a .NET SDK (`dotnet`) on PATH; not needed for the
    /// baseline setup at all.
    fn install_or_update_razor(
        worktree: &zed::Worktree,
        language_server_id: &LanguageServerId,
    ) -> Result<Option<Vec<String>>> {
        let user_settings = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings);

        let Some(user_settings) = user_settings else {
            return Ok(None);
        };

        let Some(roslyn_root) = user_settings["roslyn_source_repository_root"]
            .as_str()
            .map(str::to_string)
        else {
            return Ok(None);
        };

        let (sdk_path, sdk_version) = Self::find_dotnet_sdk_path(worktree)?;

        let directory_exists = zed::process::Command::new("test")
            .arg("-d")
            .arg(&roslyn_root)
            .output()?
            .status
            == Some(0);

        if directory_exists {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::CheckingForUpdate,
            );

            let reset = zed::process::Command::new("git")
                .arg("-C")
                .arg(&roslyn_root)
                .arg("reset")
                .arg("--hard")
                .output()?;
            if reset.status != Some(0) {
                return Err("failed to reset the roslyn_source_repository_root git checkout — is `git` installed?".to_string());
            }
            // A failed pull (e.g. offline) isn't fatal — keep using what's there.
            let _ = zed::process::Command::new("git")
                .arg("-C")
                .arg(&roslyn_root)
                .arg("pull")
                .output();

            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );
        } else {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            let clone = zed::process::Command::new("git")
                .arg("clone")
                .arg("https://github.com/dotnet/roslyn")
                .arg("--branch")
                .arg("release/stable")
                .arg(&roslyn_root)
                .output()?;

            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );

            if clone.status != Some(0) {
                return Err("failed to clone dotnet/roslyn into roslyn_source_repository_root (git + internet access required for this opt-in setting)".to_string());
            }
        }

        let env_vars = worktree.shell_env();
        let build = zed::process::Command::new("dotnet")
            .arg("build")
            .arg(format!(
                "{roslyn_root}/src/Razor/src/Razor/src/Microsoft.VisualStudioCode.RazorExtension/Microsoft.VisualStudioCode.RazorExtension.csproj"
            ))
            .arg("--configuration")
            .arg("Release")
            .envs(env_vars)
            .output()?;

        if build.status != Some(0) {
            let stdout = String::from_utf8_lossy(&build.stdout);
            let stderr = String::from_utf8_lossy(&build.stderr);
            return Err(format!(
                "failed to build Microsoft.VisualStudioCode.RazorExtension from roslyn_source_repository_root (status: {:?})\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
                build.status
            ));
        }

        let sdk_version_short = format!(
            "net{}",
            sdk_version.split('.').take(2).collect::<Vec<_>>().join(".")
        );
        let extension_dll = format!(
            "{roslyn_root}/artifacts/bin/Microsoft.VisualStudioCode.RazorExtension/Release/{sdk_version_short}/Microsoft.VisualStudioCode.RazorExtension.dll"
        );

        Ok(Some(vec!["--extension".to_string(), extension_dll]))
    }

    // Format: "10.0.100 [/home/user/.dotnet/sdk]" — one line per installed SDK.
    fn find_dotnet_sdk_path(worktree: &zed::Worktree) -> Result<(String, String)> {
        let env_vars = worktree.shell_env();
        let output = zed::process::Command::new("dotnet")
            .arg("--list-sdks")
            .envs(env_vars)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let last_line = stdout.lines().last().ok_or_else(|| {
            format!("no .NET SDK found (`dotnet --list-sdks` returned nothing): {stdout}")
        })?;

        let parts: Vec<&str> = last_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(format!(
                "unable to parse `dotnet --list-sdks` output: {stdout}"
            ));
        }

        let version = parts[0].to_string();
        let base_path = parts[1].trim_matches(|c| c == '[' || c == ']').to_string();
        Ok((format!("{base_path}/{version}"), version))
    }
}

enum ServerPath {
    Exe(String),
    Dll(String),
}

impl ServerPath {
    fn as_str(&self) -> &str {
        match self {
            ServerPath::Exe(path) | ServerPath::Dll(path) => path,
        }
    }
}
