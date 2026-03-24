use zed_extension_api::{self as zed, Result};

struct StarExtension;

impl zed::Extension for StarExtension {
    fn new() -> Self {
        StarExtension
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // Look for `star` on PATH, or use a workspace-local binary
        let path = worktree
            .which("star")
            .ok_or_else(|| "star binary not found on PATH. Install it with: cargo install --path .".to_string())?;

        Ok(zed::Command {
            command: path,
            args: vec!["lsp".to_string()],
            env: Default::default(),
        })
    }
}

zed::register_extension!(StarExtension);
