mod args;

use args::{ConnectSubCommand, CreateSubCommand, EC2connector, EntityType, StopSubCommand};

use aws_config;
use aws_config::BehaviorVersion;
use aws_sdk_ec2::{
    types::Filter, types::InstanceStateName, types::SummaryStatus, Client as EC2Client,
};
use aws_sdk_ec2instanceconnect::{
    Client as InstanceConnectClient, Error as InstanceConnectClientError,
};
use chrono::format::strftime::StrftimeItems;
use chrono::Utc;
use clap::Parser;
use dirs;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
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
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "",
                )) as Box<dyn Error>);
            }
            match connect_command.command {
                ConnectSubCommand::Ec2(ec2_connect_command) => {
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
                        Ok((instance_id, public_dns, is_running)) => {
                            let mut public_dns = public_dns;
                            if is_running {
                                println!("Instance is already running, connecting...");
                                connect_to_instance(instance_id.clone(), public_key.clone())
                                    .await?;
                            } else {
                                println!("Instance not running...");
                                let mut input = String::new();
                                print!("Do you want to start the instance? (y/n):");
                                io::stdout().flush()?; // Make sure the prompt is immediately displayed
                                io::stdin().read_line(&mut input)?;

                                let input = input.trim(); // Remove trailing newline

                                match input {
                                    "y" => {
                                        println!("Starting instance...");
                                        let config = aws_config::load_defaults(
                                            BehaviorVersion::v2023_11_09(),
                                        )
                                        .await;
                                        let client = EC2Client::new(&config);

                                        let start_resp = client
                                            .start_instances()
                                            .instance_ids(instance_id.clone())
                                            .send()
                                            .await;

                                        match start_resp {
                                            Ok(_) => {
                                                println!("Waiting for instance to be in running state. May take a few minutes...");

                                                let spinner_chars = vec!['|', '/', '-', '\\'];
                                                let mut spinner_index = 0;

                                                // Polling loop
                                                loop {
                                                    let check_resp = client
                                                        .describe_instance_status()
                                                        .instance_ids(instance_id.clone())
                                                        .include_all_instances(true)
                                                        .send()
                                                        .await;

                                                    if let Ok(status_resp) = check_resp {
                                                        let instance_statuses =
                                                            status_resp.instance_statuses();

                                                        if let Some(instance_status) =
                                                            instance_statuses.first()
                                                        {
                                                            let is_running = matches!(
                                                                instance_status
                                                                    .instance_state()
                                                                    .and_then(|s| s.name()),
                                                                Some(InstanceStateName::Running)
                                                            );

                                                            let system_status_ok = matches!(
                                                                instance_status
                                                                    .system_status()
                                                                    .and_then(|s| s.status()),
                                                                Some(SummaryStatus::Ok)
                                                            );

                                                            let instance_status_ok = matches!(
                                                                instance_status
                                                                    .instance_status()
                                                                    .and_then(|s| s.status()),
                                                                Some(SummaryStatus::Ok)
                                                            );

                                                            if is_running
                                                                && system_status_ok
                                                                && instance_status_ok
                                                            {
                                                                println!("\nInstance is now running and has passed status checks.");
                                                                break;
                                                            }
                                                        }
                                                    }

                                                    // Print spinner character
                                                    print!(
                                                        "\r{}{}{}{}{}",
                                                        spinner_chars[spinner_index],
                                                        spinner_chars[spinner_index],
                                                        spinner_chars[spinner_index],
                                                        spinner_chars[spinner_index],
                                                        spinner_chars[spinner_index]
                                                    );
                                                    io::stdout().flush().unwrap();

                                                    // Update spinner index for next character
                                                    spinner_index =
                                                        (spinner_index + 1) % spinner_chars.len();

                                                    // Wait for some time before the next poll
                                                    tokio::time::sleep(
                                                        std::time::Duration::from_millis(200),
                                                    )
                                                    .await;
                                                }

                                                connect_to_instance(
                                                    instance_id.clone(),
                                                    public_key.clone(),
                                                )
                                                .await?;

                                                let dns_resp = client
                                                    .describe_instances()
                                                    .instance_ids(instance_id.clone())
                                                    .send()
                                                    .await;

                                                match dns_resp {
                                                    Ok(resp) => {
                                                        if let Some(reservation) =
                                                            resp.reservations().first()
                                                        {
                                                            if let Some(instance) =
                                                                reservation.instances().first()
                                                            {
                                                                if let Some(dns) =
                                                                    instance.public_dns_name()
                                                                {
                                                                    if !dns.is_empty() {
                                                                        public_dns =
                                                                            dns.to_string(); // Clone the DNS name
                                                                        println!("Public DNS of the instance: {}", public_dns);
                                                                    } else {
                                                                        eprintln!(
                                                                            "Public DNS is empty."
                                                                        );
                                                                    }
                                                                } else {
                                                                    eprintln!("Failed to retrieve public DNS.");
                                                                }
                                                            } else {
                                                                eprintln!("Instance not found in the response.");
                                                            }
                                                        } else {
                                                            eprintln!("No reservations found in the response.");
                                                        }
                                                    }
                                                    Err(e) => {
                                                        eprintln!("Failed to describe instances for public DNS retrieval: {}", e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to start instance: {}", e);
                                                return Err(Box::new(std::io::Error::new(
                                                    std::io::ErrorKind::Other,
                                                    e,
                                                ))
                                                    as Box<dyn Error>);
                                            }
                                        }
                                    }
                                    "n" => {
                                        println!("Instance not started");
                                        return Ok(());
                                    }
                                    _ => {
                                        println!("Invalid input. Please enter 'y' or 'n'.");
                                        return Ok(());
                                    }
                                };
                            }

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
                                writeln!(ssh_config, "\nHost {}", host_name.clone())?;
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
            if !is_configured() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "",
                )) as Box<dyn Error>);
            }
            match create_command.command {
                CreateSubCommand::NewEc2 => {
                    // prompt user for name, size (small, medium, large), git repo and branch
                }
                CreateSubCommand::CopyOf(create_copy_of_command) => {
                    println!("Creating copy of ec2: {:?}", create_copy_of_command);
                }
            }
        }
        EntityType::Stop(stop_command) => {
            if !is_configured() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "",
                )) as Box<dyn Error>);
            }
            match stop_command.command {
                StopSubCommand::Ec2(ec2_stop_command) => {
                    
                    // get ec2 public dns address and id
                    // stop ec2
                    // remove ssh config entry

                    match get_instance_info(&ec2_stop_command.ec2_name).await {
                        Ok((instance_id, _, is_running)) => {
                            if is_running {
                                let config =
                                    aws_config::load_defaults(BehaviorVersion::v2023_11_09()).await;
                                let client = EC2Client::new(&config);

                                let stop_resp = client
                                    .stop_instances()
                                    .instance_ids(instance_id.clone())
                                    .send()
                                    .await;

                                match stop_resp {
                                    Ok(_) => {
                                        println!(
                                            "Successfully sent stop request for instance {}",
                                            ec2_stop_command.ec2_name
                                        );
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to stop instance: {}", e);
                                        return Err(Box::new(std::io::Error::new(
                                            std::io::ErrorKind::Other,
                                            e,
                                        ))
                                            as Box<dyn Error>);
                                    }
                                }
                            } else {
                                println!("Instance is not in a state to be stopped.");
                            }
                        }
                        Err(e) => {
                            eprintln!("Error getting instance info: {}", e);
                            return Err(
                                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                                    as Box<dyn Error>,
                            );
                        }
                    }
                }
                StopSubCommand::Neptune(neptune_stop_command) => {
                    println!("Stopping Neptune: {:?}", neptune_stop_command);
                }
            }
        }
        EntityType::Config(config_command) => {
            // if !is_configured() {
            //     return Err(Box::new(std::io::Error::new(
            //         std::io::ErrorKind::Other,
            //         "",
            //     )) as Box<dyn Error>);
            // }
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

// Returns instance id, public dns, and a boolean indicating if the instance is running
async fn get_instance_info(instance_name: &str) -> Result<(String, String, bool), String> {
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

    for reservation in resp.reservations() {
        for instance in reservation.instances() {
            let instance_id = instance.instance_id().unwrap_or_default().to_string();
            let public_dns = instance.public_dns_name().unwrap_or_default().to_string();

            let is_running = instance
                .state()
                .and_then(|s| s.name())
                .map_or(false, |state_name| {
                    *state_name == InstanceStateName::Running
                });

            return Ok((instance_id, public_dns, is_running));
        }
    }

    Err("No matching instances found".to_string())
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
        println!("AWS credentials are not configured, please install the AWS CLI or run 'aws configure'");
        return false
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
        return false
    }

   true
}


