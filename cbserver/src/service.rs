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
        let r = match r.host_info {
            Some(host_info) => host_info,
            None => {
                return Err(tonic::Status::new(
                    tonic::Code::InvalidArgument,
                    String::from("no nested host_info"),
                ))
            }
        };
        let mut conn = self.get_conn().map_err(status_from_error)?;

        conn.exec_drop(
            "INSERT INTO hosts (id, hostname, info, alloc_state, health_state) VALUES (UUID_TO_BIN(UUID()), :hostname, :info, :alloc_state, :health_state)",
            params! { "hostname" => r.hostname, "info" => r.info, "alloc_state" => r.alloc_state, "health_state" => r.health_state },
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
                "SELECT id, hostname, info, alloc_state, health_state FROM hosts WHERE hostname = :hostname",
                params! { "hostname" => r.hostname.as_str() },
            )
            .map_err(status_from_error)?;

        match row {
            Some(row) => {
                let (id, hostname, info, alloc_state, health_state) =
                    mysql::from_row::<(Vec<u8>, String, String, u8, u8)>(row);
                let uid = uuid::Uuid::from_slice(id.as_slice()).map_err(status_from_error)?;

                Ok(tonic::Response::new(cbprotolib::GetHostInfoResponse {
                    host_info: std::option::Option::Some(cbprotolib::HostInfo {
                        id: uid.to_string(),
                        hostname: hostname,
                        info: info,
                        alloc_state: alloc_state as i32,
                        health_state: health_state as i32,
                    }),
                }))
            }
            None => Err(tonic::Status::not_found(format!(
                "host {} not found",
                r.hostname
            ))),
        }
    }

    async fn list_hosts(
        &self,
        _: tonic::Request<cbprotolib::ListHostsRequest>,
    ) -> Result<tonic::Response<cbprotolib::ListHostsResponse>, tonic::Status> {
        let mut conn = self.get_conn().map_err(status_from_error)?;

        let host_infos = conn
            .exec_map(
                "SELECT id, hostname, info, alloc_state, health_state FROM hosts",
                (), // no query params
                |(id, hostname, info, alloc_state, health_state): (
                    Vec<u8>,
                    String,
                    String,
                    u8,
                    u8,
                )| {
                    // This is a bit sketchy, we return a nil (all 0) uuid if the parse fails.
                    // In production code we should detect this case and raise an alert/error.
                    let uid_str = uuid::Uuid::from_slice(id.as_slice())
                        .unwrap_or(uuid::Uuid::nil())
                        .to_string();
                    cbprotolib::HostInfo {
                        id: uid_str,
                        hostname: hostname,
                        info: info,
                        alloc_state: alloc_state as i32,
                        health_state: health_state as i32,
                    }
                },
            )
            .map_err(status_from_error)?;

        Ok(tonic::Response::new(cbprotolib::ListHostsResponse {
            host_infos: host_infos,
        }))
    }
}
