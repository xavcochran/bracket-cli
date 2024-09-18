use rpassword::read_password;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use toml;

use crate::utils::AppError;

#[derive(Serialize, Deserialize)]
struct AWSConfig {
    public_key: String,
    #[serde(skip)]
    secret_key: String,
}

pub async fn setup_aws() -> Result<(), AppError> {
    let mut config = AWSConfig {
        public_key: String::new(),
        secret_key: String::new()
    };
    print!("Enter your AWS Personal Access Token: ");
    io::stdout().flush().unwrap();
    config.pat = read_password().unwrap();

    print!("Enter your AWS username: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut config.username).unwrap();
    config.username = config.username.trim().to_string();



    // Store or use the credentials as needed
    println!("Username: {}", config.username);
    println!("Email: {}", config.email);
    // println!("PAT: {}", config.pat);
    store_aws_config(config);

    return Ok(());
}

pub async fn list_aws_config() -> Result<(), AppError> {
    let config_path = dirs::config_dir()
        .unwrap()
        .join("bracket/aws_config.toml");
    // read aws_config.toml file and print username and email
    let file = fs::read_to_string(config_path)?;

    let config: AWSConfig = toml::from_str(&file).map_err(|e| AppError::Other(format!("Could not read AWS configuration file: {}", e)))?;
    
    println!("Username: {}", config.username);
    println!("Email Address: {}", config.email);

    return Ok(());
}

fn stor_aws_config(config: AWSConfig) {
    let config_path = dirs::config_dir()
        .unwrap()
        .join("bracket/aws_config.toml");
    let toml = toml::to_string(&config).unwrap();
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(config_path, toml).unwrap();
}
