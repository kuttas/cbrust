use std::sync::Mutex;
mod service;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let url = "mysql://root@localhost:3306/compute_broker";
    let connection_pool = mysql::Pool::new(url)?;

    let address = "[::1]:8080".parse().unwrap();
    let cbservice = service::ComputeBrokerService::new(Mutex::new(Box::new(connection_pool)));

    tonic::transport::Server::builder()
        .add_service(cbprotolib::compute_broker_server::ComputeBrokerServer::new(
            cbservice,
        ))
        .serve(address)
        .await?;

    Ok(())
}
