use aws_config;
use aws_config::BehaviorVersion;
use aws_sdk_cloudwatch::{types::Dimension, types::Statistic, Client as CloudWatchClient};
use aws_sdk_ec2::{types::Filter, types::InstanceStateName, Client as EC2Client};
use aws_sdk_ec2instanceconnect::{
    Client as InstanceConnectClient, Error as InstanceConnectClientError,
};
use aws_sdk_neptune::Client as NeptuneClient;
use std::time::SystemTime;

use chrono::{self};
use clap::Parser;

use regex::Regex;
use std::error::Error;

use dialoguer::{theme::ColorfulTheme, Select};
use std::io::{self, Write};
use std::process::Command;
use tokio::task;

pub async fn list_ec2() -> Result<(), Box<dyn Error>> {
    // list all ec2 instances
    // get instance id, public dns, and state
    // print out the info
    let config = aws_config::load_defaults(BehaviorVersion::v2023_11_09()).await;
    let client = EC2Client::new(&config);
    let cw_client = CloudWatchClient::new(&config);

    let resp = match client.describe_instances().send().await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Failed to describe instances: {}", e);
            return Err(
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn Error>
            );
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
            let cpu_utilization = get_cpu_utilization(&cw_client, &instance_id)
                .await
                .unwrap_or(0.0);
            instances.push((name, is_running, instance_id, public_dns, cpu_utilization));
        }
    }

    if instances.is_empty() {
        println!("No instances found");
        return Ok(());
    } else {
        println!(" ");
        let title = "EC2 INSTANCE INFORMATION";
        let separator = "=".repeat(104);
        let name = "\x1b[1m".to_owned() + title + "\x1b[0m";
        let lines = "\x1b[1m=\x1b[0m".repeat(90);

        println!("{:^1$}", name, separator.len());
        println!("{}", lines);
        println!("{}", " ");

        println!(
            "{:<20} {:<10} {:<20} {:<10} {:<20}",
            "Name", "Status", "Instance ID", "CPU Utilization", "Public DNS",
        );
        println!("{}", "-".repeat(90));
        for (name, is_running, instance_id, public_dns, cpu_utilization) in instances {
            println!(
                "{:<20} {:<10} {:<20} {:<10} {:<20} ",
                name,
                if is_running { "running" } else { "stopped" },
                instance_id,
                format!("{:.2}%", cpu_utilization),
                public_dns,
            );
        }
        return Ok(());
    }
}


async fn get_cpu_utilization(cw_client: &CloudWatchClient, instance_id: &str) -> Result<f64, Box<dyn Error>> {
    let dimension = Dimension::builder()
        .name("InstanceId")
        .value(instance_id)
        .build();

    let statistic = Statistic::Average;

    let chrono_start_time = chrono::Utc::now() - chrono::Duration::minutes(5);
    let chrono_end_time = chrono::Utc::now();

    let start_time = aws_sdk_ec2::primitives::DateTime::from(SystemTime::from(chrono_start_time));
    let end_time = aws_sdk_ec2::primitives::DateTime::from(SystemTime::from(chrono_end_time));

    let resp = cw_client.get_metric_statistics()
        .namespace("AWS/EC2")
        .metric_name("CPUUtilization")
        .dimensions(dimension)
        .start_time(start_time)
        .end_time(end_time)
        .period(300)
        .statistics(statistic)
        .send()
        .await?;

    let average_cpu_utilization = resp.datapoints().get(0).and_then(|dp| dp.average()).unwrap_or(0.0);

    Ok(average_cpu_utilization)
}