use cbprotolib::compute_broker_client::ComputeBrokerClient;
use clap::{Args, Parser, Subcommand};

/// Command-Line Interface (CLI) for Compute Broker
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Lists hosts in Compute Broker's database
    ListHosts(ListHosts),

    /// Gets detailed info about a host in Compute Broker's database
    GetHostInfo(GetHostInfo),

    /// Adds hosts to Compute Broker's database
    AddHost(AddHost),

    /// Flags a healthy host for maintenance
    MaintainHost(MaintainHost),
}

#[derive(Args)]
struct ListHosts {}

#[derive(Args)]
struct GetHostInfo {
    /// Fully qualified domain name of host, e.g. `foobar.example.com`
    hostname: String,
}

#[derive(Args)]
struct AddHost {
    /// Fully qualified domain name of host, e.g. `foobar.example.com`
    hostname: String,
    /// A string of useful info about the host
    #[arg(default_value_t = String::from("<no info provided>"))]
    info: String,
}

#[derive(Args)]
struct MaintainHost {
    /// Fully qualified domain name of host, e.g. `foobar.example.com`
    hostname: String,
}

fn print_host_info(host_info: &cbprotolib::HostInfo) {
    println!(
        "Host: {}\nID: {}\nInfo: {}\nAllocation State: {}\nHealth State: {}\n",
        host_info.hostname,
        host_info.id,
        host_info.info,
        cbprotolib::HostAllocationState::from_i32(host_info.alloc_state)
            .unwrap_or(cbprotolib::HostAllocationState::Invalid)
            .as_str_name(),
        cbprotolib::HostHealthState::from_i32(host_info.health_state)
            .unwrap_or(cbprotolib::HostHealthState::Invalid)
            .as_str_name(),
    );
}

async fn list_hosts() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ComputeBrokerClient::connect("http://[::1]:8080").await?;
    let request = tonic::Request::new(cbprotolib::ListHostsRequest {});
    let response = client.list_hosts(request).await?.into_inner();
    for host_info in response.host_infos.iter() {
        print_host_info(host_info);
    }

    Ok(())
}

async fn get_host_info(args: GetHostInfo) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ComputeBrokerClient::connect("http://[::1]:8080").await?;
    let hostname = args.hostname.trim();
    let request = tonic::Request::new(cbprotolib::GetHostInfoRequest {
        hostname: String::from(hostname),
    });
    let response = client.get_host_info(request).await?.into_inner();
    let host_info = match response.host_info {
        Some(host_info) => host_info,
        None => return Err("no nested host_info".into()),
    };
    print_host_info(&host_info);
    Ok(())
}

async fn add_host(args: AddHost) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ComputeBrokerClient::connect("http://[::1]:8080").await?;
    let hostname = args.hostname.trim();
    let info = args.info.trim();
    let request = tonic::Request::new(cbprotolib::AddHostRequest {
        host_info: std::option::Option::Some(cbprotolib::HostInfo {
            id: String::from(""), // TODO: this is janky, but making it optional also adds annoying complexity
            hostname: String::from(hostname),
            info: String::from(info),
            alloc_state: cbprotolib::HostAllocationState::Unallocated as i32,
            health_state: cbprotolib::HostHealthState::Good as i32,
        }),
    });
    let _ = client.add_host(request).await?;
    println!("added host '{}'", hostname);
    Ok(())
}

async fn maintain_host(args: MaintainHost) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ComputeBrokerClient::connect("http://[::1]:8080").await?;
    let hostname = args.hostname.trim();
    let request = tonic::Request::new(cbprotolib::MaintainHostRequest {
        hostname: String::from(hostname),
    });
    let _ = client.maintain_host(request).await?.into_inner();
    println!("flagged host '{}' for maintenance", hostname);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    match args.command {
        Commands::ListHosts(_) => list_hosts().await?,
        Commands::GetHostInfo(args) => get_host_info(args).await?,
        Commands::AddHost(args) => add_host(args).await?,
        Commands::MaintainHost(args) => maintain_host(args).await?,
    }
    Ok(())
}
