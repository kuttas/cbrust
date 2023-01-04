use mysql::prelude::Queryable;
use std::sync::{Arc, Mutex};
mod service;

// This "maintainer" reconciliation loop could be split to another process/service or sharded.
// If sharded, then it needs to 'lease' the hosts to prevent multiple services from maintaining the same host.
async fn maintainer(pool: Arc<Mutex<Box<mysql::Pool>>>) {
    loop {
        // Pause 1 sec between polling DB
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        let mut conn = pool
            .lock()
            .unwrap()
            .get_conn()
            .expect("expected connection"); // TODO: easiest to crash & restart on db connection fail?

        conn.exec_map(
            "SELECT hostname FROM hosts WHERE health_state=2",
            (), // no query params
            |hostname: String| {
                println!("host {} is in maintenance", hostname);
            },
        )
        .expect("expected sql success"); // TODO: probably shouldn't panic here, though it may be better to crash & restart?
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let url = "mysql://root@localhost:3306/compute_broker";
    let connection_pool = Arc::new(Mutex::new(Box::new(mysql::Pool::new(url)?)));

    let address = "[::1]:8080".parse().unwrap();
    let cbservice = service::ComputeBrokerService::new(connection_pool.clone());

    let cloned_pool = connection_pool.clone();
    let maintainer_task = tokio::task::spawn(async { maintainer(cloned_pool).await });
    let handler_task = tonic::transport::Server::builder()
        .add_service(cbprotolib::compute_broker_server::ComputeBrokerServer::new(
            cbservice,
        ))
        .serve(address);

    let (result1, result2) = futures::join!(maintainer_task, handler_task);
    // TODO: is there a more idiomatic way to handle both errors?
    result1?;
    result2?;
    Ok(())
}
