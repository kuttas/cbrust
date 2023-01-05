use mysql::{params, prelude::Queryable};
use std::sync::{Arc, Mutex};
mod service;
use tokio_util::sync::CancellationToken;

// This "maintainer" reconciliation loop could be split to another process/service or sharded.
// If sharded, then it needs to 'lease' the hosts to prevent multiple services from maintaining the same host.
async fn maintainer(pool: Arc<Mutex<Box<mysql::Pool>>>, cancel_token: CancellationToken) {
    loop {
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_millis(1000)) => {
                let mut conn = pool.lock().unwrap().get_conn().expect("expected connection"); // TODO: easiest to crash & restart on db connection fail?
                conn.exec_map(
                    "SELECT hostname FROM hosts WHERE health_state=:state",
                    params! { "state" => cbprotolib::HostHealthState::InMaintenance as i32 },
                    |hostname: String| {
                        println!("host {} is in maintenance", hostname);
                    },
                )
                .expect("expected sql success"); // TODO: probably shouldn't panic here, though it may be better to crash & restart?
            }
            _ = cancel_token.cancelled() => {
                println!("terminating maintainer");
                return;
            }
        }
    }
}

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
    let maintainer_task = tokio::task::spawn(async { maintainer(cloned_pool, cloned_token).await });

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
