
use std::fmt;

use aws_config;
use aws_config::BehaviorVersion;
use aws_sdk_ec2::{
    types::Filter, types::InstanceStateName, Client as EC2Client,
};



// Returns instance id, public dns, and a boolean indicating if the instance is running
pub async fn get_instance_info(instance_name: &str) -> Result<(String, String, bool), String> {
    let config = aws_config::load_defaults(BehaviorVersion::v2024_03_28()).await;
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


#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    CommandFailed(String),
    ConfigurationError(String),
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::CommandFailed(cmd) => write!(f, "Command failed: {}", cmd),
            AppError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            AppError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}