use crate::args::version;
use crate::utils::AppError;
use chrono::format;
use colored::Colorize;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use reqwest;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::process::Command;


pub fn config_cli() -> Result<(), AppError> {
    let ide_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select your IDE")
        .default(0)
        .item("VSCode")
        .item("IntelliJ")
        .interact()
        .unwrap();

    let user_os = std::env::consts::OS;

    match ide_selection {
        0 => {
            // VSCode
            //check what os is running
            //check vscode is installed
            //download vscode cli for that os
            let vscode_exists = Command::new("code")
                .arg("--version")
                .output()
                .map_err(|e| {
                    AppError::CommandFailed(format!("Failed to run 'code --version': {}", e))
                })?;

            if !vscode_exists.status.success() {
                let (vscode_cli_url, input) = match user_os {
                    "macos" => (
                        "https://code.visualstudio.com/docs/setup/mac",
                        "Cmd+Shift+P",
                    ),
                    "windows" => (
                        "https://code.visualstudio.com/docs/setup/windows",
                        "Ctrl+Shift+P",
                    ),
                    "linux" => (
                        "https://code.visualstudio.com/docs/setup/linux",
                        "Ctrl+Shift+P",
                    ),
                    _ => ("https://code.visualstudio.com/docs/setup", "Ctrl+Shift+P"),
                };
                return Err(AppError::CommandFailed(
                    format!("VSCode or the VSCode CLI is not installed. \n
                    Please install VSCode if you haven't already, and install the VSCode CLI by doing the following:\n
                    \t1. Press {} to open the command palette\n 
                    \t2. Type 'Shell Command: Install 'code' command in PATH' and press Enter\n
                    \t3. Restart your terminal\n
                    \t4. Run \t`bracket config cli`\t again\n

                    For more information, visit: {}\n
                     ", input, vscode_cli_url),
                ));
            }
        }
        1 => {
            // IntelliJ
            // //check what os is running
            // //check intelliJ is installed
            // //download jetbrains gateway for that os
            // let jetbrains_gateway_exists = Command::new()

            return Err(AppError::CommandFailed(
                "IntelliJ is not supported yet".to_string(),
            ));
        }
        _ => {
            return Err(AppError::CommandFailed(
                "Invalid input! Please input 1 or 2".to_string(),
            ));
        }
    }

    // let vscode_exists = Command::new("bash")
    //     .arg("-c")
    //     .arg("code --version")
    //     .output()
    //     .map_err(|e| AppError::CommandFailed(format!("Failed to run 'code --version': {}", e)))?;

    return Ok(());
}

#[derive(Serialize, Deserialize, Debug)]
struct GitHubReleaseResponse {
    tag_name: String,
}

pub async fn check_for_new_version() -> Result<(), AppError> {
    let client = reqwest::Client::new();
    let latest_version_response = client
        .get("https://api.github.com/repos/bracketengineering/bracket-cli/releases/latest")
        .header("User-Agent", "Bracket CLI")
        .send()
        .await;

    let latest_version = match latest_version_response {
        Ok(response) => match serde_json::from_str(&response.text().await.unwrap()) {
            Ok(GitHubReleaseResponse { tag_name }) => GitHubReleaseResponse { tag_name },

            Err(e) => {
                return Err(AppError::Other(format!(
                    "Failed to parse latest version response: {}",
                    e
                )))
            }
        },
        Err(e) => {
            return Err(AppError::Other(format!(
                "Failed to get latest version: {}",
                e
            )))
        }
    };

    let current_semver = Version::parse(version::VERSION.trim().trim_start_matches('v'))
        .map_err(|e| AppError::Other(format!("Invalid current version format: {}", e)))?;

    let latest_semver = Version::parse(latest_version.tag_name.trim().trim_start_matches('v'))
        .map_err(|e| AppError::Other(format!("Invalid latest version format: {}", e)))?;

    if current_semver < latest_semver {
        println!(
            "\n
        {}\n
        Run {} to update to the latest version.\n\n",
            format!(
                "You are running version {} but version {} is available",
                version::VERSION.trim().yellow(),
                latest_version.tag_name.trim().green()
            )
            .bold()
            .underline(),
            "bracket update".green().bold()
        );
    }

    Ok(())
}

pub async fn cli_update() -> Result<(), AppError> {
    let user_os = std::env::consts::OS;

    let client = reqwest::Client::new();
    let latest_version_response = client
        .get("https://api.github.com/repos/bracketengineering/bracket-cli/releases/latest")
        .header("User-Agent", "Bracket CLI")
        .send()
        .await;

    let latest_version = match latest_version_response {
        Ok(response) => match serde_json::from_str(&response.text().await.unwrap()) {
            Ok(GitHubReleaseResponse { tag_name }) => GitHubReleaseResponse { tag_name },
            Err(e) => {
                return Err(AppError::Other(format!(
                    "Failed to parse latest version response: {}",
                    e
                )))
            }
        },
        Err(e) => {
            return Err(AppError::Other(format!(
                "Failed to get latest version: {}",
                e
            )))
        }
    };

    let current_semver = Version::parse(version::VERSION.trim().trim_start_matches('v'))
        .map_err(|e| AppError::Other(format!("Invalid current version format: {}", e)))?;

    let latest_semver = Version::parse(latest_version.tag_name.trim().trim_start_matches('v'))
        .map_err(|e| AppError::Other(format!("Invalid latest version format: {}", e)))?;

    if current_semver < latest_semver {
        println!(
            "Updating bracket CLI to version {}",
            latest_version.tag_name
        );
        let update_output = match user_os {
            "macos" | "linux" => {
                let output = Command::new("sh")
                    .arg("-c")
                    .arg("curl -sSL https://raw.githubusercontent.com/bracketengineering/bracket-cli/refs/heads/main/install/install.sh | bash")
                    .output()
                    .map_err(|e| {
                        AppError::CommandFailed(format!("Failed to run update command on {}: {}", user_os, e))
                    })?;

                // Check if the command executed successfully
                if !output.status.success() {
                    return Err(AppError::CommandFailed(format!(
                        "Update command failed with exit code: {}\nStdout: {}\nStderr: {}",
                        output.status.code().unwrap_or(-1),
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    )));
                }

                output
            }
            "windows" => {
                let output = Command::new("powershell")
                    .arg("-Command")
                    .arg("Invoke-WebRequest -Uri https://raw.githubusercontent.com/bracketengineering/bracket-cli/refs/heads/main/install/install.bat -OutFile install.bat; ./install.bat")
                    .output()
                    .map_err(|e| {
                        AppError::CommandFailed(format!("Failed to run update command on {}: {}", user_os, e))
                    })?;

                if !output.status.success() {
                    return Err(AppError::CommandFailed(format!(
                        "Update command failed with exit code: {}\nStdout: {}\nStderr: {}",
                        output.status.code().unwrap_or(-1),
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    )));
                }

                output
            }
            _ => return Err(AppError::CommandFailed(format!("Invalid OS: {}", user_os))),
        };

        match update_output.status.success() {
            true => {
                println!(
                    "Bracket CLI updated successfully. {}",  "Please restart your terminal.".bold().yellow().italic()
                );
            }
            false => {
                return Err(AppError::CommandFailed(format!(
                    "Failed to update bracket CLI: {}",
                    String::from_utf8_lossy(&update_output.stderr)
                )))
            }
        }
    } else {
        println!(
            "\n{}\n",
            "You are running the latest version of the Bracket CLI. ✅".bold()
        ); // prints ✅
    }

    Ok(())
}


pub async fn config_golang (version: Version) -> Result<(), AppError> {
    let user_os = std::env::consts::OS;

    let go_exists = Command::new("go")
        .arg("version")
        .output()
        .map_err(|e| {
            AppError::CommandFailed(format!("Failed to run 'go version': {}", e))
        })?;

    match go_exists.status.success() {
        true => {
            println!("Go is installed");
        }
        false => {
            match user_os {
                "macos" => {
                    let output = Command::new("sh")
                        .arg("-c")
                        .arg("brew install go")
                        .output()
                        .map_err(|e| {
                            AppError::CommandFailed(format!("Failed to install Go on macOS: {}", e))
                        })?;

                    if !output.status.success() {
                        return Err(AppError::CommandFailed(format!(
                            "Failed to install Go on macOS: {}",
                            String::from_utf8_lossy(&output.stderr)
                        )));
                    }
                }
                "linux" | "al2023" => {
                    let output = Command::new("sh")
                        .arg("-c")
                        .arg("yum install golang-go | y")
                        .output()
                        .map_err(|e| {
                            AppError::CommandFailed(format!("Failed to install Go on Linux: {}", e))
                        })?;

                    if !output.status.success() {
                        return Err(AppError::CommandFailed(format!(
                            "Failed to install Go on Linux: {}",
                            String::from_utf8_lossy(&output.stderr)
                        )));
                    }
                }
                "windows" => {
                    return Err(AppError::CommandFailed(
                        "Go is not installed, please go to 'https://go.dev/doc/install' to install Go for windows".to_string(),
                    ));
                }
                _ => {
                    return Err(AppError::CommandFailed(format!(
                        "Go is not installed and we don't have instructions for installing it on {}",
                        user_os
                    )))
                }
            }
        }
    }

    Ok(())
}