use cbprotolib::{compute_broker_client::ComputeBrokerClient, AddHostRequest};
use std::io::stdin;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ComputeBrokerClient::connect("http://[::1]:8080").await?;
    loop {
        println!("\nPlease enter a hostname");
        let mut hostname = String::new();
        stdin().read_line(&mut hostname).unwrap();
        let hostname = hostname.trim();

        // TODO: fix the CLI to use actual flags/options so we can do CRUD operations.
        // let request = tonic::Request::new(GetHostInfoRequest {
        //     hostname: String::from(hostname),
        // });
        // let response = client.get_host_info(request).await?;
        // println!("CB service says: '{}'", response.into_inner().info);

        // Add host to DB with bogus info string
        let request = tonic::Request::new(AddHostRequest {
            hostname: String::from(hostname),
            info: String::from("todo useful info"),
        });
        let response = client.add_host(request).await?;
        println!("CB service success: {:?}", response);
    }
}
