use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

mod maintainer;
mod service;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let url = "mysql://root@localhost:3306/compute_broker";
    let connection_pool = Arc::new(Mutex::new(Box::new(mysql::Pool::new(url)?)));

    let address = "[::1]:8080".parse().unwrap();
    let cbservice = service::ComputeBrokerService::new(connection_pool.clone());

    let token = CancellationToken::new();

    // Start maintenance reconciliation loop
    let cloned_pool = connection_pool.clone();
    let cloned_token = token.clone();
    let maintainer_task = tokio::task::spawn(async {
        maintainer::reconciliation_loop(cloned_pool, cloned_token).await
    });

    // Start gRPC handler task
    let handler_task = tonic::transport::Server::builder()
        .add_service(cbprotolib::compute_broker_server::ComputeBrokerServer::new(
            cbservice,
        ))
        .serve_with_shutdown(address, async {
            // Handle SIGINT / CTRL+C gracefully
            tokio::signal::ctrl_c()
                .await
                .expect("failed to listen for event");
            token.cancel();
        });

    //
    let (result1, result2) = futures::join!(maintainer_task, handler_task);
    // TODO: is there a more idiomatic way to handle both errors?
    result1?;
    result2?;
    println!("shutdown gracefully");

    Ok(())
}
