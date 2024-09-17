mod args;
mod ec2;
mod github;
mod neptune;
mod utils;

use args::{
    ConnectSubCommand, CreateSubCommand, EC2connector, EntityType, SetupSubCommand, StopSubCommand,
};
use aws_config;
use clap::Parser;
use regex::Regex;
use std::error::Error;
use std::io::{self, Write};
use std::process::Command;

use std::fmt;

use utils::AppError;

#[::tokio::main]
async fn main() -> Result<(), AppError> {
    let code_check = Command::new("bash")
        .arg("-c")
        .arg("code --version")
        .output()
        .map_err(|e| AppError::CommandFailed(format!("Failed to run 'code --version': {}", e)))?;

    if !code_check.status.success() {
        return Err(AppError::CommandFailed(
            "'code' command is not available".to_string(),
        ));
    }

    let args = EC2connector::parse();

    match args.entity_type {
        EntityType::Connect(connect_command) => {
            if !is_configured()? {
                return Err(AppError::ConfigurationError(
                    "AWS or GitHub not configured.".to_string(),
                ));
            }
            match connect_command.command {
                ConnectSubCommand::Ec2(ec2_connect_command) => {
                    ec2::connect::ec2_connect(ec2_connect_command).await?;
                }
                ConnectSubCommand::Neptune => {
                    println!("Connect to Neptune");
                }
            }
        }

        EntityType::Create(create_command) => {
            if !is_configured()? {
                return Err(AppError::ConfigurationError(
                    "AWS or GitHub not configured.".to_string(),
                ));
            }
            match create_command.command {
                CreateSubCommand::NewEc2 => {
                    ec2::create::create_new_ec2().await;
                }
                CreateSubCommand::CopyOf(create_copy_of_command) => {
                    println!("Creating copy of ec2: {:?}", create_copy_of_command);
                }
            }
        }

        EntityType::Stop(stop_command) => {
            if !is_configured()? {
                return Err(AppError::ConfigurationError(
                    "AWS or GitHub not configured.".to_string(),
                ));
            }
            match stop_command.command {
                StopSubCommand::Ec2(ec2_stop_command) => {
                    ec2::stop::stop_ec2(ec2_stop_command).await?;
                }
                StopSubCommand::Neptune(neptune_stop_command) => {
                    println!("Stopping Neptune: {:?}", neptune_stop_command);
                }
            }
        }

        EntityType::Setup(config_command) => match config_command.command {
            SetupSubCommand::Aws => {
                let command = "aws configure";
                let child = Command::new("bash")
                    .arg("-c")
                    .arg(&command)
                    .spawn()
                    .map_err(AppError::Io)?;

                let output = child.wait_with_output().map_err(AppError::Io)?;

                if !output.status.success() {
                    return Err(AppError::CommandFailed(
                        "Failed to configure AWS.".to_string(),
                    ));
                }
            }
            SetupSubCommand::Github => {
                github::setup::setup_github().await?;
            }
        },

        EntityType::List(list_command) => match list_command.command {
            args::ListSubCommand::Ec2 => {
                ec2::list::list_ec2().await?;
            }
            args::ListSubCommand::Neptune => {
                neptune::list::list_neptune().await?;
            }
            args::ListSubCommand::Github => {
                github::setup::list_github_config().await?;
            }
        },
    }

    Ok(())
}

pub fn is_configured() -> Result<bool, AppError> {
    let aws_check_cmd = "aws configure get aws_access_key_id && aws configure get region";
    let aws_output = Command::new("bash")
        .arg("-c")
        .arg(aws_check_cmd)
        .output()
        .map_err(|e| {
            AppError::CommandFailed(format!("Failed to check AWS configuration: {}", e))
        })?;

    if !aws_output.status.success() {
        return Err(AppError::ConfigurationError(
            "AWS credentials are not configured".to_string(),
        ));
    }

    let github_check_cmd = "gh auth status";
    let github_output = Command::new("bash")
        .arg("-c")
        .arg(github_check_cmd)
        .output()
        .map_err(|e| {
            AppError::CommandFailed(format!("Failed to check GitHub configuration: {}", e))
        })?;

    if !github_output.status.success() {
        return Err(AppError::ConfigurationError(
            "GitHub credentials are not configured".to_string(),
        ));
    }

    Ok(true)
}
