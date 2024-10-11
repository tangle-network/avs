use color_eyre::eyre::{eyre, Result};
use gadget_sdk::executor::process::manager::GadgetProcessManager;
use std::os::unix::fs::PermissionsExt;

pub const TANGLE_AVS_ASCII: &str = r#"
 _____   _     _    _  _____ _      _____
|_   _| / \    | \ | |/ ____| |    |  ___|
  | |  / _ \   |  \| | |  __| |    | |__
  | | / ___ \  | . ` | | |_ | |    |  __|
  | |/ /   \ \ | |\  | |__| | |____| |___
  |_/_/     \_\|_| \_|\_____|______|_____|

              _   __     __ ____
             / \  \ \   / // ___|
            / _ \  \ \ / / \__ \
           / ___ \  \ V /  ___) |
          /_/   \_\  \_/  |____/
"#;

/// Fetches and runs the Tangle validator binary, initiating a validator node.
///
/// # Process
/// 1. Checks for the existence of the binary.
/// 2. If not found, downloads it from the official Tangle GitHub release page.
/// 3. Ensures the binary has executable permissions.
/// 4. Executes the binary to start the validator node.
///
/// # Errors
/// Returns an error if:
/// - The binary download fails
/// - Setting executable permissions fails
/// - The binary execution fails
pub async fn run_tangle_validator() -> Result<()> {
    let mut manager = GadgetProcessManager::new();

    // Check if the binary exists
    if !std::path::Path::new("tangle-default-linux-amd64").exists() {
        let install = manager
            .run("binary_install".to_string(), "wget https://github.com/webb-tools/tangle/releases/download/v1.0.0/tangle-default-linux-amd64")
            .await
            .map_err(|e| eyre!(e.to_string()))?;
        manager
            .focus_service_to_completion(install)
            .await
            .map_err(|e| eyre!(e.to_string()))?;
    }

    // Check if the binary is executable
    let metadata = std::fs::metadata("tangle-default-linux-amd64")?;
    let permissions = metadata.permissions();
    if !permissions.mode() & 0o111 != 0 {
        let make_executable = manager
            .run(
                "make_executable".to_string(),
                "chmod +x tangle-default-linux-amd64",
            )
            .await
            .map_err(|e| eyre!(e.to_string()))?;
        manager
            .focus_service_to_completion(make_executable)
            .await
            .map_err(|e| eyre!(e.to_string()))?;
    }

    // Start the validator
    let start_validator = manager
        .run(
            "tangle_validator".to_string(),
            "./tangle-default-linux-amd64",
        )
        .await
        .map_err(|e| eyre!(e.to_string()))?;
    manager
        .focus_service_to_completion(start_validator)
        .await
        .map_err(|e| eyre!(e.to_string()))?;

    Ok(())
}
