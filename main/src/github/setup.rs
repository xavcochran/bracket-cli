use rpassword::read_password;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use toml;

use crate::utils::AppError;

#[derive(Serialize, Deserialize)]
struct GitHubConfig {
    username: String,
    email: String,
    #[serde(skip)]
    pat: String,
}

pub async fn setup_github() -> Result<(), AppError> {
    let mut config = GitHubConfig {
        username: String::new(),
        email: String::new(),
        pat: String::new(),
    };
    print!("Enter your GitHub Personal Access Token: ");
    io::stdout().flush().unwrap();
    config.pat = read_password().unwrap();

    print!("Enter your GitHub username: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut config.username).unwrap();
    config.username = config.username.trim().to_string();

    print!("Enter your GitHub email address: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut config.email).unwrap();
    config.email = config.email.trim().to_string();

    // Store or use the credentials as needed
    println!("Username: {}", config.username);
    println!("Email: {}", config.email);
    // println!("PAT: {}", config.pat);
    store_github_config(config);

    return Ok(());
}

pub async fn list_github_config() -> Result<(), AppError> {
    let config_path = dirs::config_dir()
        .unwrap()
        .join("bracket/github_config.toml");
    // read github_config.toml file and print username and email
    let file = fs::read_to_string(config_path)?;

    let config: GitHubConfig = toml::from_str(&file).map_err(|e| AppError::Other(format!("Could not read GitHub configuration file: {}", e)))?;
    
    println!("Username: {}", config.username);
    println!("Email Address: {}", config.email);

    return Ok(());
}

fn store_github_config(config: GitHubConfig) {
    let config_path = dirs::config_dir()
        .unwrap()
        .join("bracket/github_config.toml");
    let toml = toml::to_string(&config).unwrap();
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(config_path, toml).unwrap();
}
