use std::io::{self, Write};
use std::process::Command;
use dialoguer::{theme::ColorfulTheme, Select};
use tokio::task;

pub async fn create_new_ec2(){
    // prompt user for name, size (small, medium, large), git repo and branch
    let mut name = String::new();
    let mut size = String::new();
    let mut git_repo = String::new();
    let mut branch = String::new();

    print!("Enter the name of the EC2 instance: ");
    io::stdout().flush().unwrap(); // Flush stdout to ensure the prompt is printed before read_line
    io::stdin().read_line(&mut name).unwrap();
    name = name.trim().to_string();


    let repos = task::spawn_blocking(|| get_github_repos()).await.unwrap();
    let repo_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a GitHub repo")
        .default(0)
        .items(&repos[..])
        .interact()
        .unwrap();

    let selected_repo = repos[repo_selection].clone();


    let branches = task::spawn_blocking(|| get_github_branches(selected_repo)).await.unwrap();
    let branch_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a branch")
        .default(0)
        .items(&branches[..])
        .interact()
        .unwrap();

    let selected_branch = branches[branch_selection].clone();


}


// need to use sign in with git auth rather than personal access token
fn get_github_repos() -> Vec<String> {
    let output = Command::new("gh")
        .arg("repo")
        .arg("list")
        .output()
        .expect("Failed to list GitHub repos");

    let repos = String::from_utf8(output.stdout).unwrap();
    repos.lines().map(|line| line.split('\t').next().unwrap().to_string()).collect()
}

fn get_github_branches(repo: String) -> Vec<String> {
    let output = Command::new("gh")
        .arg("api")
        .arg(format!("/repos/{}/branches", repo))
        .output()
        .expect("Failed to list GitHub branches");

    let branches = String::from_utf8(output.stdout).unwrap();
    let branches: Vec<serde_json::Value> = serde_json::from_str(&branches).unwrap();
    branches.into_iter().map(|branch| branch["name"].as_str().unwrap().to_string()).collect()
}