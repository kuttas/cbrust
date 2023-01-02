use mysql::{params, prelude::Queryable};
use std::sync::Mutex;

// Convert `Error` (common in our gRPC handlers) to `tonic::Status` (expected gRPC handler return).
// TODO: Should fill in useful status codes, which is probably why this conversion is not implicit!
fn status_from_error<T: std::error::Error + Send + Sync + 'static>(err: T) -> tonic::Status {
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
        let mut conn = self.get_conn().map_err(status_from_error)?;

        conn.exec_drop(
            "INSERT INTO hosts (id, hostname, info) VALUES (UUID_TO_BIN(UUID()), :hostname, :info)",
            params! { "hostname" => r.hostname, "info" => r.info },
        )
        .map_err(status_from_error)?;

        Ok(tonic::Response::new(cbprotolib::AddHostResponse {}))
    }

    async fn get_host_info(
        &self,
        request: tonic::Request<cbprotolib::GetHostInfoRequest>,
    ) -> Result<tonic::Response<cbprotolib::GetHostInfoResponse>, tonic::Status> {
        let r = request.into_inner();
        let mut conn = self.get_conn().map_err(status_from_error)?;
        let row = conn
            .exec_first(
                "SELECT id, hostname, info FROM hosts WHERE hostname = :hostname",
                params! { "hostname" => r.hostname.as_str() },
            )
            .map_err(status_from_error)?;

        match row {
            Some(row) => {
                let (id, hostname, info) = mysql::from_row::<(Vec<u8>, String, String)>(row);
                let uid = uuid::Uuid::from_slice(id.as_slice()).map_err(status_from_error)?;

                Ok(tonic::Response::new(cbprotolib::GetHostInfoResponse {
                    id: uid.to_string(),
                    hostname: hostname,
                    info: info,
                }))
            }
            None => Err(tonic::Status::not_found(format!(
                "host {} not found",
                r.hostname
            ))),
        }
    }
}
