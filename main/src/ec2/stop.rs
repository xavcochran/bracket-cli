use crate::aws_config;
use crate::args;
use crate::utils::get_instance_info;
use aws_config::BehaviorVersion;
use aws_sdk_ec2::Client as EC2Client;
use crate::AppError;

pub async fn stop_ec2(ec2_stop_command: args::Ec2StopCommand) -> Result<(), AppError> {
    // get ec2 public dns address and id
    // stop ec2
    // remove ssh config entry

    match get_instance_info(&ec2_stop_command.ec2_name).await {
        Ok((instance_id, _, is_running)) => {
            if is_running {
                let config = aws_config::load_defaults(BehaviorVersion::v2024_03_28()).await;
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
                        return Ok(());
                    }
                    Err(e) => {
                        let err_str: String = format!("Failed to connect to instance: {}", e);
                        return Err(AppError::CommandFailed(err_str));
                    }
                }
            } else {
                println!("Instance is not in a state to be stopped.");
                return Ok(()); // instance is not running, so we don't need to stop it
            }
        }
        Err(e) => {
            let err_str: String = format!("Error getting instance info: {}", e);
            return Err(AppError::CommandFailed(err_str));
        }
    }
}
