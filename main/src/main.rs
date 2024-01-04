mod args;

use args::{
    ConfigSubCommand, ConnectSubCommand, CreateSubCommand, EC2connector, EntityType, StopSubCommand,
};

use std::time::SystemTime;
use aws_config;
use aws_config::BehaviorVersion;
use aws_sdk_ec2::{
    types::Filter, types::InstanceStateName, types::SummaryStatus, Client as EC2Client,
};
use aws_sdk_ec2instanceconnect::{
    Client as InstanceConnectClient, Error as InstanceConnectClientError,
};
use aws_sdk_neptune::Client as NeptuneClient;
use aws_sdk_cloudwatch::{Client as CloudWatchClient, types::Statistic};

use chrono::format::strftime::StrftimeItems;
use chrono::{self, Utc};
use clap::Parser;
use dirs;
use regex::Regex;
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
                return Err(
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, "")) as Box<dyn Error>
                );
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
                return Err(
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, "")) as Box<dyn Error>
                );
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
                return Err(
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, "")) as Box<dyn Error>
                );
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
                let config = aws_config::load_defaults(BehaviorVersion::v2023_11_09()).await;
                let client = EC2Client::new(&config);

                let resp = match client.describe_instances().send().await {
                    Ok(resp) => resp,
                    Err(e) => {
                        eprintln!("Failed to describe instances: {}", e);
                        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                            as Box<dyn Error>);
                    }
                };

                let mut instances = Vec::new();

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
                        let name = instance
                            .tags()
                            .iter()
                            .find(|tag| tag.key().unwrap_or_default() == "Name")
                            .map_or("".to_string(), |tag| {
                                tag.value().unwrap_or_default().to_string()
                            });
                        instances.push((name, is_running, instance_id, public_dns));
                    }
                }

                if instances.is_empty() {
                    println!("No instances found");
                } else {
                    println!(" ");
                    let title = "EC2 INSTANCE INFORMATION";
                    let separator = "=".repeat(78);
                    let name = "\x1b[1m".to_owned() + title + "\x1b[0m";
                    let lines = "\x1b[1m=\x1b[0m".repeat(70);
                    
                    println!("{:^1$}", name, separator.len());
                    println!("{}", lines);
                    println!("{}", " ");

                    println!(
                        "{:<20} {:<10} {:<20} {:<20}",
                        "Name", "Status", "Instance ID", "Public DNS"
                    );
                    println!("{}", "-".repeat(70));
                    for (name, is_running, instance_id, public_dns) in instances {
                        println!(
                            "{:<20} {:<10} {:<20} {:<20}",
                            name,
                            if is_running { "running" } else { "stopped" },
                            instance_id,
                            public_dns
                        );
                    }
                }
            }
            args::ListSubCommand::Neptune => {
                let config = aws_config::load_defaults(BehaviorVersion::v2023_11_09()).await;
                let client = NeptuneClient::new(&config);
                let cloudwatch_client = CloudWatchClient::new(&config);
                // Describe Neptune clusters
                let clusters = match client.describe_db_clusters().send().await {
                    Ok(resp) => resp,
                    Err(e) => {
                        eprintln!("Failed to describe clusters: {}", e);
                        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn Error>);
                    }
                }; 

                println!("{}", " ");
                let title = "NEPTUNE CLUSTER INFORMATION";
                let separator = "=".repeat(63);
                let lines = "\x1b[1m=\x1b[0m".repeat(55);
                println!("{:^1$}", format!("\x1b[1m{}\x1b[0m", title), separator.len());
                println!("{}", lines);
                println!("{}", " ");
                for cluster in clusters.db_clusters() {
                    
                    // Get CPU Utilization from CloudWatch
                    let metric_name = "CPUUtilization";
                    let namespace = "AWS/Neptune";

                    let chrono_start_time = chrono::Utc::now() - chrono::Duration::hours(1);
                    let chrono_end_time = chrono::Utc::now();

                    let start_time = aws_sdk_ec2::primitives::DateTime::from(SystemTime::from(chrono_start_time));
                    let end_time = aws_sdk_ec2::primitives::DateTime::from(SystemTime::from(chrono_end_time));

                    let cpu_util_resp = cloudwatch_client.get_metric_statistics()
                        .namespace(namespace)
                        .metric_name(metric_name)
                        .start_time(start_time) // Last 1 hour
                        .end_time(end_time)
                        .period(300) // 5 minutes periods
                        .statistics(Statistic::Average)
                        .dimensions(
                            aws_sdk_cloudwatch::types::Dimension::builder()
                                .name("DBClusterIdentifier")
                                .value(cluster.db_cluster_identifier().unwrap_or_default())
                                .build()
                        )
                        .send()
                        .await;
                    
                    let mut cpu_util: Option<(String, f64)> = None;
                    if let Ok(stats) = cpu_util_resp {
                        for point in stats.datapoints() {
                            match (point.timestamp(), point.average()) {
                                (Some(timestamp), Some(average)) => {
                                    let timestamp_str = timestamp.to_string();
                                    match chrono::DateTime::parse_from_rfc3339(&timestamp_str) {
                                        Ok(datetime) => {
                                            let formatted_timestamp = datetime.format("%I:%M%p %d/%m/%Y").to_string();
                                            cpu_util = Some((formatted_timestamp, average));
                                        }
                                        Err(e) => {
                                            eprintln!("Failed to parse timestamp: {}", e);
                                        }
                                    }
                                }
                                _ => {
                                    println!("Missing data");
                                }
                            }
                        }
                    } else {
                        eprintln!("Failed to get CPU utilization metrics.");
                    }
                    let cluster_name = cluster.db_cluster_identifier().unwrap_or_default();
                    let status = cluster.status().unwrap_or_default();
                    let endpoint = cluster.endpoint().unwrap_or_default();

                    let instance_count = cluster.db_cluster_members().len();

                    // Construct the AWS console link for the cluster
                    let cluster_link = format!("https://console.aws.amazon.com/neptune/home?region={}#database:id={};is-cluster=true", 
                    config.region().unwrap().as_ref(), 
                    cluster.db_cluster_identifier().unwrap_or_default());
                    println!("\x1b[1m{} {}\x1b[0m", "Cluster:", cluster_name);
                    println!("{}", "\x1b[1m-\x1b[0m".repeat(55));
                    println!("\x1b[1m{:<16}\x1b[0m {}", "Instance Count:", instance_count);
                    println!("\x1b[1m{:<16}\x1b[0m {}", "Status:", status);
                    println!("\x1b[1m{:<16}\x1b[0m {}", "CPU Utilisation:", match cpu_util {
                        Some((timestamp, average)) => format!("{:.2}% at {}", average, timestamp),
                        None => "N/A".to_string(),
                    });
                    println!("\x1b[1m{:<16}\x1b[0m {}", "Endpoint:", endpoint);
                    println!("\x1b[1m{:<16}\x1b[0m {}", "Cluster Link:", cluster_link);
                    println!("{}", " ");
                    
                }
            }
        },
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

