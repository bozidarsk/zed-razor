mod language_servers;

use std::env;

use language_servers::Roslyn;
use zed_extension_api::{self as zed, LanguageServerId, Result};

const LANGSERVERS_PACKAGE: &str = "@zed-industries/vscode-langservers-extracted";

struct Razor {
    langservers_ready: bool,
    roslyn: Option<Roslyn>,
}

impl Razor {
    /// Installs/updates the shared npm package that bundles the HTML and CSS
    /// language servers (only needs to happen once per session).
    fn ensure_langservers_extracted(
        &mut self,
        language_server_id: &LanguageServerId,
    ) -> Result<()> {
        if self.langservers_ready {
            return Ok(());
        }

        let installed = zed::npm_package_installed_version(LANGSERVERS_PACKAGE)?;
        let latest = zed::npm_package_latest_version(LANGSERVERS_PACKAGE)?;

        if installed.as_deref() != Some(latest.as_str()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            zed::npm_install_package(LANGSERVERS_PACKAGE, &latest)?;
        }

        self.langservers_ready = true;
        Ok(())
    }

    fn bundled_binary_command(
        &mut self,
        language_server_id: &LanguageServerId,
        binary_name: &str,
        relative_path: &str,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // Prefer a copy already on PATH (e.g. installed globally by the user)
        // over downloading our own.
        if let Some(path) = worktree.which(binary_name) {
            return Ok(zed::Command {
                command: path,
                args: vec!["--stdio".to_string()],
                env: Default::default(),
            });
        }

        self.ensure_langservers_extracted(language_server_id)?;

        let script_path = env::current_dir()
            .map_err(|e| e.to_string())?
            .join(relative_path)
            .to_string_lossy()
            .to_string();

        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args: vec![script_path, "--stdio".to_string()],
            env: Default::default(),
        })
    }
}

impl zed::Extension for Razor {
    fn new() -> Self {
        Self {
            langservers_ready: false,
            roslyn: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        match language_server_id.as_ref() {
            "html-in-razor" => self.bundled_binary_command(
                language_server_id,
                "vscode-html-language-server",
                "node_modules/@zed-industries/vscode-langservers-extracted/bin/vscode-html-language-server",
                worktree,
            ),
            "css-in-razor" => self.bundled_binary_command(
                language_server_id,
                "vscode-css-language-server",
                "node_modules/@zed-industries/vscode-langservers-extracted/bin/vscode-css-language-server",
                worktree,
            ),
            Roslyn::LANGUAGE_SERVER_ID => {
                let roslyn = self.roslyn.get_or_insert_with(Roslyn::new);
                roslyn.language_server_cmd(language_server_id, worktree)
            }
            id => Err(format!("unknown language server: {id}")),
        }
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        if language_server_id.as_ref() == Roslyn::LANGUAGE_SERVER_ID {
            return Roslyn::configuration_options(worktree);
        }
        Ok(None)
    }
}

zed::register_extension!(Razor);
