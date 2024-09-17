use aws_config;
use aws_config::BehaviorVersion;
use aws_sdk_cloudwatch::{types::Dimension, types::Statistic, Client as CloudWatchClient};
use aws_sdk_ec2::{types::InstanceStateName, types::SummaryStatus, Client as EC2Client};
use aws_sdk_ec2instanceconnect::{
    Client as InstanceConnectClient, Error as InstanceConnectClientError,
};
use chrono::format::strftime::StrftimeItems;
use chrono::{self, Utc};
use dirs;
use std::time::SystemTime;

use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::process::Command;

use crate::args;
use crate::utils::get_instance_info;
use crate::utils::AppError;

pub async fn ec2_connect(ec2_connect_command: args::Ec2ConnectCommand) -> Result<(), AppError> {
    // run ssh keygen command
    let file_name = "key_rsa"; // replace with your desired file name
    let home_dir = dirs::home_dir().expect("Could not get home directory");
    let ec2_connector_dir = home_dir.join("ec2_connector");

    // Create the directory if it doesn't exist
    fs::create_dir_all(&ec2_connector_dir).expect("Failed to create directory");

    let output_path = ec2_connector_dir.join(file_name);

    // Generate SSH key pair
    let _ssh_key = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "echo y | ssh-keygen -t rsa -N '' -f {}",
            output_path.to_str().expect("Failed to convert path to str")
        ))
        .output()
        .expect("Failed to generate SSH key pair");

    // Read the public key
    let file_name = "key_rsa.pub"; // replace with your desired file name
    let home_dir = dirs::home_dir().expect("Could not get home directory");
    let file_path = home_dir.join("ec2_connector").join(file_name);
    let public_key = fs::read_to_string(file_path).expect("Failed to read SSH public key file");

    // get ec2 public dns address and id
    match get_instance_info(&ec2_connect_command.ec2_name).await {
        Ok((instance_id, public_dns, is_running)) => {
            let mut public_dns = public_dns;
            // If the instance is not running, start it
            if is_running {
                println!("Instance is already running, connecting...");
                connect_to_instance(instance_id.clone(), public_key.clone()).await?;
            } else {
                println!("Instance not running...");
                let mut input = String::new();
                print!("Do you want to start the instance? (y/n):");
                io::stdout().flush()?; // Make sure the prompt is immediately displayed
                io::stdin().read_line(&mut input)?;

                let input = input.trim(); // Remove trailing newline

                // Handle user input
                match input {
                    // if the user enters 'y', start the instance
                    "y" => {
                        println!("Starting instance...");
                        // Create a new EC2 client
                        let config =
                            aws_config::load_defaults(BehaviorVersion::v2023_11_09()).await;
                        let client = EC2Client::new(&config);

                        // Start the instance
                        let start_resp = client
                            .start_instances()
                            .instance_ids(instance_id.clone())
                            .send()
                            .await;

                        // Check if the instance was started successfully
                        match start_resp {
                            Ok(_) => {
                                println!("Waiting for instance to be in running state. May take a few minutes...");

                                let spinner_chars = vec!['|', '/', '-', '\\'];
                                let mut spinner_index = 0;

                                // Polling loop
                                loop {
                                    // Check instance status
                                    let check_resp = client
                                        .describe_instance_status()
                                        .instance_ids(instance_id.clone())
                                        .include_all_instances(true)
                                        .send()
                                        .await;

                                    if let Ok(status_resp) = check_resp {
                                        let instance_statuses = status_resp.instance_statuses();

                                        if let Some(instance_status) = instance_statuses.first() {
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

                                            if is_running && system_status_ok && instance_status_ok
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
                                    spinner_index = (spinner_index + 1) % spinner_chars.len();

                                    // Wait for some time before the next poll
                                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                                }

                                // Connect to the instance
                                connect_to_instance(instance_id.clone(), public_key.clone())
                                    .await?;

                                // Get the public DNS of the instance
                                let dns_resp = client
                                    .describe_instances()
                                    .instance_ids(instance_id.clone())
                                    .send()
                                    .await;

                                match dns_resp {
                                    Ok(resp) => {
                                        if let Some(reservation) = resp.reservations().first() {
                                            if let Some(instance) = reservation.instances().first()
                                            {
                                                if let Some(dns) = instance.public_dns_name() {
                                                    if !dns.is_empty() {
                                                        public_dns = dns.to_string(); // Clone the DNS name
                                                        println!(
                                                            "Public DNS of the instance: {}",
                                                            public_dns
                                                        );
                                                    } else {
                                                        eprintln!("Public DNS is empty.");
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
                                let err_str: String = format!("Failed to start instance: {}", e);
                                return Err(AppError::CommandFailed(err_str));
                            }
                        }
                    }
                    // if the user enters 'n', exit the program
                    "n" => {
                        println!("Instance not started");
                        return Ok(());
                    }
                    // if the user enters anything else, prompt them again
                    _ => {
                        println!("Invalid input. Please enter 'y' or 'n'.");
                        return Ok(());
                    }
                };
            }

            // Check if entry with public DNS already exists in SSH config
            let current_datetime = Utc::now()
                .format_with_items(StrftimeItems::new("%d-%m-%Y-%H.%M"))
                .to_string();
            let host_name = format!(
                "ec2Connector-{}-{}",
                ec2_connect_command.ec2_name, current_datetime
            );
            let ssh_config_path = dirs::home_dir()
                .ok_or_else(|| AppError::Other("Could not find home directory".to_string()))?
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

            // If entry does not exist, append the new configuration to the SSH config file
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
                "code --folder-uri vscode-remote://ssh-remote+{}/home/ec2-user/",
                host_name
            );

            let _output = Command::new("bash")
                .arg("-c")
                .arg(&command)
                .output()
                .expect("Failed to execute command");

            let mut ssh_config_contents = fs::read_to_string(&ssh_config_path)?;
            let entry_start = ssh_config_contents.find(&format!("\nHost {}", host_name));
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
            let err_str: String = format!("Failed to connect to instance: {}", e);
            return Err(AppError::CommandFailed(err_str));
        }
    }
    return Ok(());
}

// Connect to an EC2 instance using EC2 Instance Connect
async fn connect_to_instance(instance_id: String, ssh_public_key: String) -> Result<(), AppError> {
    let config = aws_config::load_defaults(BehaviorVersion::v2024_03_28()).await;
    let client = InstanceConnectClient::new(&config);

    client
        .send_ssh_public_key()
        .instance_id(&instance_id)
        .ssh_public_key(&ssh_public_key)
        .instance_os_user("ec2-user")
        .send()
        .await
        .expect("Could not connect to instance. Please try again!");

    Ok(())
}

// Get the average CPU utilization of an EC2 instance over the last 5 minutes
async fn get_cpu_utilization(
    cw_client: &CloudWatchClient,
    instance_id: &str,
) -> Result<f64, Box<dyn Error>> {
    let dimension = Dimension::builder()
        .name("InstanceId")
        .value(instance_id)
        .build();

    let statistic = Statistic::Average;

    let chrono_start_time = chrono::Utc::now() - chrono::Duration::minutes(5);
    let chrono_end_time = chrono::Utc::now();

    let start_time = aws_sdk_ec2::primitives::DateTime::from(SystemTime::from(chrono_start_time));
    let end_time = aws_sdk_ec2::primitives::DateTime::from(SystemTime::from(chrono_end_time));

    let resp = cw_client
        .get_metric_statistics()
        .namespace("AWS/EC2")
        .metric_name("CPUUtilization")
        .dimensions(dimension)
        .start_time(start_time)
        .end_time(end_time)
        .period(300)
        .statistics(statistic)
        .send()
        .await?;

    let average_cpu_utilization = resp
        .datapoints()
        .get(0)
        .and_then(|dp| dp.average())
        .unwrap_or(0.0);

    Ok(average_cpu_utilization)
}
