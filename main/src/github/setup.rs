use rpassword::read_password;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use toml;

#[derive(Serialize, Deserialize)]
struct GitHubConfig {
    username: String,
    email: String,
    pat: String,
}

pub async fn setup_github() -> Result<(), Box<dyn std::error::Error>> {
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
    println!("PAT: {}", config.pat); // In real implementation, avoid printing PAT.
    store_github_config(config);

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
