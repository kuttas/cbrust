use cbservice::{compute_broker_client::ComputeBrokerClient, GetHostInfoRequest};
use std::io::stdin;

pub mod cbservice {
    tonic::include_proto!("cbservice");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ComputeBrokerClient::connect("http://[::1]:8080").await?;
    loop {
        println!("\nPlease enter a hostname");
        let mut hostname = String::new();
        stdin().read_line(&mut hostname).unwrap();
        let hostname = hostname.trim();

        let request = tonic::Request::new(GetHostInfoRequest {
            hostname: String::from(hostname),
        });
        let response = client.get_host_info(request).await?;
        println!("CB service says: '{}'", response.into_inner().info);
    }
}
