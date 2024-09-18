use clap::{Args, Parser, Subcommand};

// pub mod args;

#[derive(Debug, Parser)]
#[clap(name = "AWS Connector", version = "1.0", author = "Your Name. <")]

pub struct EC2connector {
    #[clap(subcommand)]
    pub entity_type: EntityType,
}

#[derive(Debug, Subcommand)]
pub enum EntityType {
    // /// Updates the bracket cli 
    // Update,

    /// Connects to existing EC2. Automatically starts one if none are running.
    Connect(ConnectCommand),

    /// Creates a new EC2 instance.
    Create(CreateCommand),

    /// Closes the connection to the EC2 instance and shuts it down.
    Stop(StopCommand),

    /// Setup your credentials to be able to connect to our EC2s and use them effectively.
    Setup(SetupCommand),

    /// Lists resources that are available to you.
    List(ListCommand),
}

#[derive(Debug, Args)]
pub struct ConnectCommand {
    #[clap(subcommand)]
    pub command: ConnectSubCommand,
}

#[derive(Debug, Subcommand)]
pub enum ConnectSubCommand {
    /// Connects to an existing EC2 instance. If it is not running it starts it up.
    Ec2(Ec2ConnectCommand),

    /// Creates a medium sized ec2 with a gremlin server connected to the test neptune instance.
    Neptune,
}

#[derive(Debug, Args)]
pub struct Ec2ConnectCommand {
    pub ec2_name: String,
}

#[derive(Debug, Args)]
pub struct CreateCommand {
    #[clap(subcommand)]
    pub command: CreateSubCommand,
}

#[derive(Debug, Subcommand)]
pub enum CreateSubCommand {
    /// Takes you through the process of creating a new EC2 instance.
    NewEc2,

    /// Creates a copy of an existing EC2 instance.
    CopyOf(CreateCopyOfCommand),
}

#[derive(Debug, Args)]
pub struct CreateCopyOfCommand {
    pub ec2_name: String,
}

#[derive(Debug, Args)]
pub struct SetupCommand {
    #[clap(subcommand)]
    pub command: SetupSubCommand,
}

#[derive(Debug, Subcommand)]
pub enum SetupSubCommand {
    /// Setupures your AWS credentials and other options needed.
    Aws,

    /// Setupures your Git credentials and other options needed.
    Github,
}

#[derive(Debug, Args)]
pub struct GithubSetup {
    #[clap(subcommand)]
    pub command: GithubSetupCommand,
}


#[derive(Debug, Subcommand)]
pub enum GithubSetupCommand {
    PATToken(PATTokenCommand),
    Email(EmailCommand),
    UserName(UserNameCommand),
}

#[derive(Debug, Args)]
pub struct PATTokenCommand {
    pub token: String,
}

#[derive(Debug, Args)]
pub struct EmailCommand {
    pub email: String,
}

#[derive(Debug, Args)]
pub struct UserNameCommand {
    pub username: String,
}

#[derive(Debug, Args)]
pub struct StopCommand {
    #[clap(subcommand)]
    pub command: StopSubCommand,
}

#[derive(Debug, Subcommand)]
pub enum StopSubCommand {
    /// Stops the EC2 instance and closes the connection.
    Ec2(Ec2StopCommand),

    /// Stops the Neptune instance and closes the connection.
    Neptune(NeptuneStopCommand),
}

#[derive(Debug, Args)]
pub struct Ec2StopCommand {
    pub ec2_name: String,
}

#[derive(Debug, Args)]
pub struct NeptuneStopCommand {
    pub neptune_name: String,
}


#[derive(Debug, Args)]
pub struct ListCommand {
    #[clap(subcommand)]
    pub command: ListSubCommand,
}

#[derive(Debug, Subcommand)]
pub enum ListSubCommand {
    // /// Lists all the resources available to you.
    // All(AllListCommand),
    /// Stops the EC2 instance and closes the connection.
    Ec2,

    /// Stops the Neptune instance and closes the connection.
    Neptune,

    /// Lists Github coniguration
    Github,
}

