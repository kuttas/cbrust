use mysql::{params, prelude::Queryable};
use std::sync::{Arc, Mutex};
mod service;
use futures::StreamExt;
use tokio_util::sync::CancellationToken;

async fn maintain_host(pool: Arc<Mutex<Box<mysql::Pool>>>, hostname: String) -> String {
    println!("maintaining host {}...", hostname);
    tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
    let mut conn = pool
        .lock()
        .unwrap()
        .get_conn()
        .expect("expected connection"); // TODO: probably shouldn't panic here
    let _ = conn.exec_iter(
        "UPDATE hosts SET health_state=:target_state WHERE hostname=:hostname AND health_state=:source_state;",
        params! { "hostname" => hostname.as_str(), "target_state" => cbprotolib::HostHealthState::Good as i32, "source_state" => cbprotolib::HostHealthState::InMaintenance as i32},
    ).expect("expected sql success"); // TODO: probably shouldn't panic here
    return hostname.clone();
}

async fn find_hosts_in_state(
    pool: &Arc<Mutex<Box<mysql::Pool>>>,
    health_state: cbprotolib::HostHealthState,
) -> Vec<String> {
    let mut conn = pool
        .lock()
        .unwrap()
        .get_conn()
        .expect("expected connection"); // TODO: probably shouldn't panic here
    let hostnames: std::vec::Vec<std::string::String> = conn
        .exec_map(
            "SELECT hostname FROM hosts WHERE health_state=:state",
            params! { "state" => health_state as i32 },
            |hostname: String| hostname,
        )
        .expect("expected sql success"); // TODO: probably shouldn't panic here
    return hostnames;
}

// Atomically move host from one expected state to another, and do nothing if host was not in the expected state.
// Return true if transition occurred, false otherwise.
async fn transition_host(
    pool: &Arc<Mutex<Box<mysql::Pool>>>,
    hostname: &str,
    source_state: cbprotolib::HostHealthState,
    target_state: cbprotolib::HostHealthState,
) -> bool {
    let mut conn = pool
        .lock()
        .unwrap()
        .get_conn()
        .expect("expected connection"); // TODO: probably shouldn't panic here

    let result = conn.exec_iter(
        "UPDATE hosts SET health_state=:target_state WHERE hostname=:hostname AND health_state=:source_state;",
        params! { "hostname" => hostname, "target_state" => target_state as i32, "source_state" => source_state as i32},
    ).expect("expected sql success"); // TODO: probably shouldn't panic here

    result.affected_rows() > 0 // TODO: error if != 1?
}

// This "maintainer" reconciliation loop could be split to another process/service or sharded.
// If sharded, then every host operation needs to deterministically lock or 'lease' the hosts to prevent
// multiple shards from maintaining the same host.
async fn maintainer(pool: Arc<Mutex<Box<mysql::Pool>>>, cancel_token: CancellationToken) {
    let mut fu = futures::stream::FuturesUnordered::new();

    // Scan for hosts that were in the middle of maintenance when we first started and kick off state machines.
    {
        let hostnames =
            find_hosts_in_state(&pool, cbprotolib::HostHealthState::InMaintenance).await;

        for hostname in hostnames.into_iter() {
            println!("resuming maintenance on host {}", hostname);
            let cloned_pool = pool.clone();
            fu.push(maintain_host(cloned_pool, hostname.clone()));
        }
    }

    loop {
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_millis(1000)) => {
                let hostnames = find_hosts_in_state(&pool, cbprotolib::HostHealthState::NeedsMaintenance).await;

                for hostname in hostnames.into_iter() {
                    // If we succeed in atomically moving the host from the "needs maintenance" state to the "in maintenance" state,
                    // then kick off a maintenance task.
                    if transition_host(&pool, hostname.as_str(), cbprotolib::HostHealthState::NeedsMaintenance, cbprotolib::HostHealthState::InMaintenance).await {
                        println!("triggering maintenance on host {}", hostname);
                        let cloned_pool = pool.clone();
                        fu.push(maintain_host(cloned_pool, hostname.clone()));
                    }
                }
            }
            _ = cancel_token.cancelled() => {
                println!("terminating maintainer");
                return;
            }

            // NOTE: we must match Some explicitly, because an empty FuturesUnordered will
            // immediately yield the None value.
            Some(v) = fu.next() => {
                println!("completed maintenance on host {:?}", v);
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
