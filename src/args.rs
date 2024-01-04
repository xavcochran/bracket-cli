use clap:: {
  Args,
  Parser,
  Subcommand
};

#[derive(Debug, Parser)]
#[clap(name = "My CLI Program", version = "1.0", author = "Your Name. <")]

pub struct EC2connector {
  #[clap(subcommand)]
  pub entity_type: EntityType,
}

#[derive(Debug, Subcommand)]
pub enum EntityType {
  /// Connects to existing EC2. Automatically starts one if none are running.
  Connect(ConnectCommand),

  /// Creates a new EC2 instance.
  Create(CreateCommand),

  /// Closes the connection to the EC2 instance and shuts it down.
  Stop(StopCommand),

  /// Configures your credentials to be able to connect to our EC2s and use them effectively. 
  Config(ConfigCommand),
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
  New,

  /// Creates a copy of an existing EC2 instance.
  CopyOf(CreateCopyOfCommand),
}

#[derive(Debug, Args)]
pub struct CreateCopyOfCommand {
  pub ec2_name: String,
}

#[derive(Debug, Args)]
pub struct ConfigCommand {
  #[clap(subcommand)]
  pub command: ConfigSubCommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubCommand {
  /// Configures your AWS credentials and other options needed.
  AwsConfig(AwsConfigCommand),

  /// Configures your Git credentials and other options needed.
  GitConfig(GitConfigCommand),
}

#[derive(Debug, Args)]
pub struct AwsConfigCommand {
  #[clap(short, long)]
  pub aws_access_key_id: String,

  #[clap(short, long)]
  pub aws_secret_access_key: String,

  #[clap(short, long)]
  pub aws_region: String,
}

#[derive(Debug, Args)]
pub struct GitConfigCommand {
  #[clap(short, long)]
  pub git_username: String,

  #[clap(short, long)]
  pub git_email: String,
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
  Neptune(NeptuneStopCommand)
}

#[derive(Debug, Args)]
pub struct Ec2StopCommand {
  pub ec2_name: String,
}

#[derive(Debug, Args)]
pub struct NeptuneStopCommand {
  pub neptune_name: String,
}
