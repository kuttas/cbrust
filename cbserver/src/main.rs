use mysql::prelude::Queryable;
use mysql::Pool;

use cbservice::{
    compute_broker_server::{ComputeBroker, ComputeBrokerServer},
    GetHostInfoRequest, GetHostInfoResponse,
};
use tonic::{transport::Server, Request, Response, Status};

pub mod cbservice {
    tonic::include_proto!("cbservice");
}

#[derive(Debug, Default)]
pub struct ComputeBrokerService {}

#[tonic::async_trait]
impl ComputeBroker for ComputeBrokerService {
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
    let url = "mysql://root@localhost:3306";
    let pool = Pool::new(url)?;
    let mut conn = pool.get_conn()?;

    let dbs = conn.query_map("SHOW DATABASES;", |name| Database { name })?;

    println!("SHOW DATABASES result:\n{:?}", dbs);

    let address = "[::1]:8080".parse().unwrap();
    let cbservice = ComputeBrokerService::default();

    Server::builder()
        .add_service(ComputeBrokerServer::new(cbservice))
        .serve(address)
        .await?;

    Ok(())
}
