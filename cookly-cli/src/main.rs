mod args;

use args::{ConnectSubCommand, EC2connector, EntityType};

use aws_config;
use aws_config::BehaviorVersion;
use aws_sdk_ec2::{types::Filter, Client as EC2Client};
use aws_sdk_ec2instanceconnect::{
    Client as InstanceConnectClient, Error as InstanceConnectClientError,
};
use clap::Parser;
use dirs;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};

use chrono::format::strftime::StrftimeItems;
use chrono::Utc;
use std::error::Error;
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
            match connect_command.command {
                ConnectSubCommand::Ec2(ec2_connect_command) => {
                    println!("Connect command: {:?}", ec2_connect_command);
                    // run ssh keygen command
                    let file_name = "key_rsa"; // replace with your desired file name
                    let home_dir = dirs::home_dir().expect("Could not get home directory");
                    let ec2_connector_dir = home_dir.join("ec2_connector");

                    // Create the directory if it doesn't exist
                    fs::create_dir_all(&ec2_connector_dir).expect("Failed to create directory");

                    let output_path = ec2_connector_dir.join(file_name);

                    let _ssh_key = Command::new("bash")
                        .arg("-c")
                        .arg(format!(
                            "echo y | ssh-keygen -t rsa -N '' -f {}",
                            output_path.to_str().expect("Failed to convert path to str")
                        ))
                        .output()
                        .expect("Failed to generate SSH key pair");

                    let file_name = "key_rsa.pub"; // replace with your desired file name
                    let home_dir = dirs::home_dir().expect("Could not get home directory");
                    let file_path = home_dir.join("ec2_connector").join(file_name);
                    let public_key =
                        fs::read_to_string(file_path).expect("Failed to read SSH public key file");
                    // get ec2 public dns address and id

                    match get_instance_info(&ec2_connect_command.ec2_name).await {
                        Ok((instance_id, public_dns)) => {
                            connect_to_instance(instance_id.clone(), public_key.clone()).await?;
                            let current_datetime = Utc::now()
                                .format_with_items(StrftimeItems::new("%d-%m-%Y-%H.%M"))
                                .to_string();
                            let host_name = format!(
                                "ec2Connector-{}-{}",
                                ec2_connect_command.ec2_name, current_datetime
                            );
                            let ssh_config_path = dirs::home_dir()
                                .ok_or("Could not find home directory")?
                                .join(".ssh")
                                .join("config");
                            // Read existing SSH config and check if the entry already exists
                            let entry_exists = if ssh_config_path.exists() {
                                let file = fs::File::open(&ssh_config_path)?;
                                let reader = BufReader::new(file);

                                reader.lines().any(|line| {
                                    line.ok().map_or(false, |l| {
                                        l.contains(&format!(
                                            "Host {}",
                                            format!(
                                                "ec2Connector-{}-{}",
                                                ec2_connect_command.ec2_name, current_datetime
                                            )
                                        ))
                                    })
                                })
                            } else {
                                false
                            };

                            // If entry does not exist, append the new configuration
                            if !entry_exists {
                                let mut ssh_config = OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open(&ssh_config_path)?;
                                writeln!(ssh_config, "\nHost {}", host_name)?;
                                writeln!(ssh_config, "  HostName {}", public_dns)?;
                                writeln!(
                                    ssh_config,
                                    "  IdentityFile {}/key_rsa",
                                    ec2_connector_dir.display()
                                )?;
                                writeln!(ssh_config, "  User ec2-user")?;
                            } else {
                                println!("SSH config entry already exists");
                            }

                            let command = format!(
                        "code --folder-uri vscode-remote://ssh-remote+{}/home/ec2-user/cookly/",
                        host_name
                    );

                            let _output = Command::new("bash")
                                .arg("-c")
                                .arg(&command)
                                .output()
                                .expect("Failed to execute command");

                            let mut ssh_config_contents = fs::read_to_string(&ssh_config_path)?;
                            let entry_start =
                                ssh_config_contents.find(&format!("\nHost {}", host_name));
                            if let Some(start) = entry_start {
                                let end = ssh_config_contents[start..]
                                    .find("\nHost ")
                                    .map_or_else(|| ssh_config_contents.len(), |end| start + end);
                                ssh_config_contents.replace_range(start..end, "");
                                fs::write(&ssh_config_path, ssh_config_contents)?;
                            }

                            println!("SSH connection established")
                        }
                        Err(e) => {
                            eprintln!("Error getting instance info: {}", e);
                            return Err(
                                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                                    as Box<dyn Error>,
                            );
                        }
                    }
                    return Ok(());
                }
                ConnectSubCommand::Neptune => {
                    println!("Connect to Neptune");
                }
            }
        }

        EntityType::Create(create_command) => {
            println!("Create command: {:?}", create_command);
        }
        EntityType::Exit => {
            println!("Exit command");
        }
        EntityType::Config(config_command) => {
            println!("Config command: {:?}", config_command);
        }
    }

    Ok(())
}

async fn connect_to_instance(
    instance_id: String,
    ssh_public_key: String,
) -> Result<(), InstanceConnectClientError> {
    let config = aws_config::load_defaults(BehaviorVersion::v2023_11_09()).await;
    let client = InstanceConnectClient::new(&config);

    client
        .send_ssh_public_key()
        .instance_id(&instance_id)
        .ssh_public_key(&ssh_public_key)
        .instance_os_user("ec2-user")
        .send()
        .await?;

    Ok(())
}

// Returns instance id and public dns of the ec2 instance
async fn get_instance_info(instance_name: &str) -> Result<(String, String), String> {
    let config = aws_config::load_defaults(BehaviorVersion::v2023_11_09()).await;
    let client = EC2Client::new(&config);

    let tag_filter = Filter::builder()
        .name("tag:Name")
        .values(instance_name)
        .build();

    let resp = match client.describe_instances().filters(tag_filter).send().await {
        Ok(resp) => resp,
        Err(e) => return Err(format!("Failed to describe instances: {}", e)),
    };

    // Assuming you are interested in the first instance that matches
    if let Some(reservation) = resp.reservations().first() {
        if let Some(instance) = reservation.instances().first() {
            let instance_id = instance.instance_id().unwrap_or_default().to_string();
            let public_dns = instance.public_dns_name().unwrap_or_default().to_string();
            return Ok((instance_id, public_dns));
        }
    }

    Err("Instance not found".to_string())
}
