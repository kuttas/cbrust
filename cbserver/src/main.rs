use mysql::*;
use mysql::prelude::*;

#[derive(Debug, PartialEq, Eq)]
struct Database {
    name: String
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let url = "mysql://root@localhost:3306";
    let pool = Pool::new(url)?;
    let mut conn = pool.get_conn()?;

    let dbs = conn
        .query_map(
            "SHOW DATABASES;",
            |name| {
                Database { name }
            },
        )?;


    println!("SHOW DATABASES result:\n{:?}", dbs);

    Ok(())
}
