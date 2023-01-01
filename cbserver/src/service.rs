use mysql::{params, prelude::Queryable};
use std::sync::Mutex;

// Convert mysql `Error` (common in our gRPC handlers) to `tonic::Status` (expected gRPC handler return).
fn status_from_mysql_error(err: mysql::Error) -> tonic::Status {
    tonic::Status::from_error(Box::new(err))
}

// Compute Broker Service handler
pub struct ComputeBrokerService {
    connection_pool: Mutex<Box<mysql::Pool>>,
}

// ComputeBrokerService helper methods
impl ComputeBrokerService {
    pub fn new(pool: Mutex<Box<mysql::Pool>>) -> Self {
        return ComputeBrokerService {
            connection_pool: pool,
        };
    }
    fn get_conn(&self) -> mysql::Result<mysql::PooledConn> {
        let pool = self.connection_pool.lock().unwrap();
        pool.get_conn()
    }
}

// ComputeBrokerService gRPC handlers
#[tonic::async_trait]
impl cbprotolib::compute_broker_server::ComputeBroker for ComputeBrokerService {
    async fn add_host(
        &self,
        request: tonic::Request<cbprotolib::AddHostRequest>,
    ) -> Result<tonic::Response<cbprotolib::AddHostResponse>, tonic::Status> {
        let r = request.into_inner();
        let mut conn = self.get_conn().map_err(status_from_mysql_error)?;

        conn.exec_drop(
            "INSERT INTO hosts (id, hostname, info) VALUES (UUID_TO_BIN(UUID()), :hostname, :info)",
            params! { "hostname" => r.hostname, "info" => r.info },
        )
        .map_err(status_from_mysql_error)?;
        Ok(tonic::Response::new(cbprotolib::AddHostResponse {}))
    }

    async fn get_host_info(
        &self,
        request: tonic::Request<cbprotolib::GetHostInfoRequest>,
    ) -> Result<tonic::Response<cbprotolib::GetHostInfoResponse>, tonic::Status> {
        let r = request.into_inner();
        println!("CB cli wants info for hostname '{}'", r.hostname);
        Ok(tonic::Response::new(cbprotolib::GetHostInfoResponse {
            info: format!("Here's the info for host '{}'", r.hostname),
        }))
    }
}
