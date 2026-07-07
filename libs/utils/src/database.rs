use std::error::Error;

use crate::secrets::get_secret;

pub struct DatabaseConnections {
    pub postgres: postgres::connection::Pool,
    pub redis: redis::connection::Pool,
}

pub async fn establish_connections()
-> Result<DatabaseConnections, Box<dyn Error>> {
    let db_rw_url = get_database_url().await?;
    let redis_url = get_secret("REDIS_URL").await?;

    let postgres = postgres::connection::establish_connection(db_rw_url)
        .await
        .expect("failed to connect to Postgres");

    let redis = redis::connection::establish_connection(redis_url)
        .await
        .expect("failed to connect to Redis");

    Ok(DatabaseConnections { postgres, redis })
}

pub async fn get_redis_connection()
-> Result<redis::connection::Pool, Box<dyn Error>> {
    let redis_url = get_secret("REDIS_URL").await?;
    let redis = redis::connection::establish_connection(redis_url)
        .await
        .expect("failed to connect to Redis");

    Ok(redis)
}

async fn get_database_url() -> Result<String, Box<dyn Error>> {
    get_secret("DATABASE_URL").await
}
