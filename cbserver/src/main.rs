use mysql::{params, prelude::Queryable, Pool, PooledConn};
use std::sync::Mutex;

use cbservice::{
    compute_broker_server::{ComputeBroker, ComputeBrokerServer},
    AddHostRequest, AddHostResponse, GetHostInfoRequest, GetHostInfoResponse,
};
use tonic::{transport::Server, Request, Response, Status};

pub mod cbservice {
    tonic::include_proto!("cbservice");
}

#[derive(Debug)]
pub struct ComputeBrokerService {
    conn: Mutex<Box<PooledConn>>,
}

#[tonic::async_trait]
impl ComputeBroker for ComputeBrokerService {
    async fn add_host(
        &self,
        request: Request<AddHostRequest>,
    ) -> Result<Response<AddHostResponse>, Status> {
        let r = request.into_inner();
        let mut conn = self.conn.lock().unwrap();
        conn.exec_drop(
            "INSERT INTO hosts (id, hostname, info) VALUES (UUID_TO_BIN(UUID()), :hostname, :info)",
            params! { "hostname" => r.hostname, "info" => r.info },
        )
        .expect("insert failed");
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

#[derive(Debug, PartialEq)]
struct Database {
    name: String,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let url = "mysql://root@localhost:3306/compute_broker";
    let pool = Pool::new(url)?;
    let mut conn = Box::new(pool.get_conn()?);

    let dbs = conn.query_map("SHOW DATABASES;", |name| Database { name })?;

    println!("SHOW DATABASES result:\n{:?}", dbs);

    let address = "[::1]:8080".parse().unwrap();
    let cbservice = ComputeBrokerService {
        conn: Mutex::new(conn),
    };

    Server::builder()
        .add_service(ComputeBrokerServer::new(cbservice))
        .serve(address)
        .await?;

    Ok(())
}
