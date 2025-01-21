use crate::{args, utils::AppError};
use aws_config::BehaviorVersion;
use aws_sdk_apprunner::operation::list_services::{ListServicesError, ListServicesOutput};
use aws_sdk_apprunner::operation::start_deployment::{StartDeploymentError, StartDeploymentOutput};
use aws_sdk_apprunner::types::OperationStatus;
use aws_sdk_apprunner::{Client, Error};
use tokio::time::{sleep, Duration};

pub async fn redeploy_app_runner(
    app_runner_command: args::RedeployCommand,
) -> Result<(), AppError> {
    println!(
        "Redeploying app runner: {}",
        app_runner_command.app_runner_name
    );

    // Create AWS App Runner client
    let config = aws_config::load_defaults(BehaviorVersion::v2024_03_28()).await;
    let client = Client::new(&config);

    // Get service ARN
    let service_arn = get_service_arn(&client, &app_runner_command.app_runner_name).await?;
    println!("Found service ARN: {}", service_arn);

    // Start deployment
    let operation_id = trigger_deployment(&client, &service_arn).await?;
    println!("Triggered deployment with operation ID: {}", operation_id);

    Ok(())
}

async fn get_service_arn(client: &Client, service_name: &str) -> Result<String, AppError> {
    let services: ListServicesOutput = client
        .list_services()
        .send()
        .await
        .map_err(|e| AppError::AwsSdk(format!("Failed to list services: {}", e)))?;

    let service_summary = services
        .service_summary_list()
        // .ok_or_else(|| AppError::NotFound("No services found".into()))?
        .iter()
        .find(|service| service.service_name() == Some(service_name))
        .ok_or_else(|| AppError::NotFound(format!("Service '{}' not found", service_name)))?;

    service_summary
        .service_arn()
        .map(String::from)
        .ok_or_else(|| AppError::NotFound("Service ARN not found".into()))
}

async fn trigger_deployment(client: &Client, service_arn: &str) -> Result<String, AppError> {
    let deployment: StartDeploymentOutput = client
        .start_deployment()
        .service_arn(service_arn)
        .send()
        .await
        .map_err(|e| AppError::AwsSdk(format!("Failed to start deployment: {}", e)))?;

    Ok(String::from(deployment.operation_id()))
}

