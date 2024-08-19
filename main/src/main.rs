mod args;
mod ec2;
mod utils;
mod neptune;

use args::{
    ConfigSubCommand, ConnectSubCommand, CreateSubCommand, EC2connector, EntityType, StopSubCommand,
};
use aws_config;
use clap::Parser;
use regex::Regex;
use std::error::Error;
use std::io::{self, Write};
use std::process::Command;


#[::tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let code_check = Command::new("bash")
        .arg("-c")
        .arg("code --version")
        .output();

    match code_check {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("'code' command is not available. Please add it to your PATH. \n
                To do this do the following: \n
                    > In VS Code, open the Command Palette (View > Command Palette or ((Ctrl/Cmd)+Shift+P)). \n
                    > Then enter 'shell command' to find the `Shell Command: Install 'code' command in PATH` command. \n
                    > Restart the terminal for the new $PATH value to take effect. You'll be able to type 'code .' in any folder to start editing files in that folder.
                ");
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "'code' command is not available",
                )) as Box<dyn Error>);
            }
        }
        Err(e) => {
            eprintln!("'code' command is not available. Please install Visual Studio Code and add it to your PATH.");
            return Err(Box::new(e) as Box<dyn Error>);
        }
    }

    let args = EC2connector::parse();
    match args.entity_type {
        EntityType::Connect(connect_command) => {
            if !is_configured() {
                return Err(
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, "")) as Box<dyn Error>
                );
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
            if !is_configured() {
                return Err(
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, "")) as Box<dyn Error>
                );
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
            if !is_configured() {
                return Err(
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, "")) as Box<dyn Error>
                );
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
        EntityType::Config(config_command) => {
            match config_command.command {
                ConfigSubCommand::Aws => {
                    let command = "aws configure";

                    let child = Command::new("bash")
                        .arg("-c")
                        .arg(&command)
                        .spawn()
                        .expect("Failed to execute command");

                    let output = child.wait_with_output()?;

                    if !output.status.success() {
                        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, ""))
                            as Box<dyn Error>);
                    }
                }
                ConfigSubCommand::Git(git_config_subcommand) => {
                    match git_config_subcommand.command {
                        // ...
                        args::GitConfigCommand::Email(mut email) => {
                            let email_regex =
                                Regex::new(r"^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+$")
                                    .unwrap();

                            loop {
                                if email_regex.is_match(&email.email) {
                                    let command = format!("gh config set git_protocol ssh && git config --global user.email \"{}\"", email.email);

                                    let child = Command::new("bash")
                                        .arg("-c")
                                        .arg(&command)
                                        .spawn()
                                        .expect("Failed to execute command");

                                    let output = child.wait_with_output()?;

                                    if output.status.success() {
                                        break;
                                    } else {
                                        return Err(Box::new(std::io::Error::new(
                                            std::io::ErrorKind::Other,
                                            "Failed to set email",
                                        ))
                                            as Box<dyn Error>);
                                    }
                                } else {
                                    eprintln!("Invalid email format. Please try again.");
                                }

                                print!("Enter a valid email: ");
                                io::stdout().flush()?;

                                email.email.clear();
                                io::stdin().read_line(&mut email.email)?;
                                email.email = email.email.trim().to_string();
                            }
                        }

                        args::GitConfigCommand::Name(name) => {
                            let command = format!("gh config set git_protocol ssh && git config --global user.name \"{}\"", name.name);

                            let child = Command::new("bash")
                                .arg("-c")
                                .arg(&command)
                                .spawn()
                                .expect("Failed to execute command");

                            let output = child.wait_with_output()?;

                            if !output.status.success() {
                                return Err(Box::new(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    "",
                                )) as Box<dyn Error>);
                            }
                        }
                        args::GitConfigCommand::Login => {
                            let command = "gh auth login";

                            let child = Command::new("bash")
                                .arg("-c")
                                .arg(&command)
                                .spawn()
                                .expect("Failed to execute command");

                            let output = child.wait_with_output()?;

                            if !output.status.success() {
                                return Err(Box::new(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    "",
                                )) as Box<dyn Error>);
                            }
                        }
                    }
                }
            }
        }
        EntityType::List(list_command) => match list_command.command {
            args::ListSubCommand::Ec2 => {
                ec2::list::list_ec2().await?;
            }

            args::ListSubCommand::Neptune => {
                neptune::list::list_neptune().await?;
            }
        },
    }

    Ok(())
}



pub fn is_configured() -> bool {
    // check if aws credentials are configured
    let command = format!("aws configure get aws_access_key_id && aws configure get region");

    let output = Command::new("bash")
        .arg("-c")
        .arg(&command)
        .output()
        .expect(
            "AWS credentials are not configured, please install the AWS CLI or run 'aws configure'",
        );

    if !output.status.success() {
        println!(
            "AWS credentials are not configured, please install the AWS CLI or run 'aws configure'"
        );
        return false;
    }

    // check if git credentials are configured
    let command = format!("gh auth status");

    let output = Command::new("bash")
        .arg("-c")
        .arg(&command)
        .output()
        .expect("Github credentials are not configured, please install the Github CLI or run 'gh auth login'");

    if !output.status.success() {
        println!("Github credentials are not configured, please install the Github CLI or run 'gh auth login'");
        return false;
    }

    true
}


