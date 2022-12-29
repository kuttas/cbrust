use mysql::{params, prelude::Queryable};
use std::sync::Mutex;

// Import proto generated Rust code into cbservice module
pub mod cbservice {
    tonic::include_proto!("cbservice");
}
use cbservice::{
    compute_broker_server::{ComputeBroker, ComputeBrokerServer},
    AddHostRequest, AddHostResponse, GetHostInfoRequest, GetHostInfoResponse,
};

use tonic::{transport::Server, Request, Response, Status};

// Convert mysql `Error` (common in our gRPC handlers) to gRPC `Status`.
fn status_from_mysql_error(err: mysql::Error) -> tonic::Status {
    Status::from_error(Box::new(err))
}

// Compute Broker Service handler
pub struct ComputeBrokerService {
    connection_pool: Mutex<Box<mysql::Pool>>,
}

// ComputeBrokerService helper methods
impl ComputeBrokerService {
    fn get_conn(&self) -> mysql::Result<mysql::PooledConn> {
        let pool = self.connection_pool.lock().unwrap();
        pool.get_conn()
    }
}

// ComputeBrokerService gRPC handlers
#[tonic::async_trait]
impl ComputeBroker for ComputeBrokerService {
    async fn add_host(
        &self,
        request: Request<AddHostRequest>,
    ) -> Result<Response<AddHostResponse>, Status> {
        let r = request.into_inner();
        let mut conn = self.get_conn().map_err(status_from_mysql_error)?;

        conn.exec_drop(
            "INSERT INTO hosts (id, hostname, info) VALUES (UUID_TO_BIN(UUID()), :hostname, :info)",
            params! { "hostname" => r.hostname, "info" => r.info },
        )
        .map_err(status_from_mysql_error)?;
        Ok(Response::new(AddHostResponse {}))
    }

    async fn get_host_info(
        &self,
        request: Request<GetHostInfoRequest>,
    ) -> Result<Response<GetHostInfoResponse>, Status> {
        let r = request.into_inner();
        println!("CB cli wants info for hostname '{}'", r.hostname);
        Ok(Response::new(GetHostInfoResponse {
            info: format!("Here's the info for host '{}'", r.hostname),
        }))
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let url = "mysql://root@localhost:3306/compute_broker";
    let connection_pool = mysql::Pool::new(url)?;

    let address = "[::1]:8080".parse().unwrap();
    let cbservice = ComputeBrokerService {
        connection_pool: Mutex::new(Box::new(connection_pool)),
    };

    Server::builder()
        .add_service(ComputeBrokerServer::new(cbservice))
        .serve(address)
        .await?;

    Ok(())
}
