mod args;
mod ec2;
mod github;
mod neptune;
mod utils;
mod config;

use args::{
    ConnectSubCommand, CreateSubCommand, EC2connector, EntityType, ConfigSubCommand, StopSubCommand,
};
use aws_config;
use clap::Parser;
use std::process::Command;

use utils::AppError;
use tokio;


#[tokio::main]
async fn main() -> Result<(), AppError> {
    match config::config::check_for_new_version().await {
        Ok(_) => {}
        Err(e) => eprintln!("Failed to check for new version: {}", e),  
    };

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

        EntityType::Config(config_command) => match config_command.command {
            ConfigSubCommand::Aws => {
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
            ConfigSubCommand::Github => {
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

        EntityType::Update => {
            match config::config::cli_update().await {
                Ok(_) => {}
                Err(e) => eprintln!("Failed to update: {}", e),
            }
        }
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
