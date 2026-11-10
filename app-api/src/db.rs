use deadpool_redis::{Config as RedisConfig, Connection, Pool as RedisPool, Runtime};
use log::{error, warn};
use redis::AsyncCommands;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;
use std::time::Duration;

pub type DbPool = PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub redis: Option<RedisPool>,
}

pub async fn init_db_pool() -> DbPool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgPoolOptions::new()
        .max_connections(30)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .idle_timeout(Duration::from_secs(300))
        .connect(&database_url)
        .await
        .expect("Failed to connect to Postgres database")
}

pub async fn run_migrations(pool: &PgPool) {
    let migration_sql = include_str!("../migrations/0001_create_notes.sql");
    sqlx::raw_sql(migration_sql)
        .execute(pool)
        .await
        .expect("Failed to execute initial migration");
}

pub fn init_redis_pool() -> Option<RedisPool> {
    match env::var("REDIS_URL") {
        Ok(redis_url) if !redis_url.trim().is_empty() => {
            let cfg = RedisConfig::from_url(redis_url);
            match cfg.create_pool(Some(Runtime::Tokio1)) {
                Ok(pool) => Some(pool),
                Err(err) => {
                    warn!(
                        "Failed to create Redis pool: {}. Running without cache.",
                        err
                    );
                    None
                }
            }
        }
        _ => {
            warn!("REDIS_URL not configured. Running without cache.");
            None
        }
    }
}

pub async fn get_redis_conn(redis_pool: &Option<RedisPool>) -> Option<Connection> {
    if let Some(pool) = redis_pool {
        match pool.get().await {
            Ok(conn) => Some(conn),
            Err(err) => {
                warn!("Failed to acquire Redis connection: {}", err);
                None
            }
        }
    } else {
        None
    }
}

pub async fn get_cached_string(redis_pool: &Option<RedisPool>, key: &str) -> Option<String> {
    let mut conn = get_redis_conn(redis_pool).await?;
    match conn.get::<_, Option<String>>(key).await {
        Ok(val) => val,
        Err(err) => {
            warn!("Redis GET error for key {}: {}", key, err);
            None
        }
    }
}

pub async fn set_cached_string(
    redis_pool: &Option<RedisPool>,
    key: &str,
    val: &str,
    ttl_seconds: u64,
) {
    if let Some(mut conn) = get_redis_conn(redis_pool).await {
        let res: Result<(), _> = conn.set_ex(key, val, ttl_seconds).await;
        if let Err(err) = res {
            error!("Redis SET_EX error for key {}: {}", key, err);
        }
    }
}

pub async fn invalidate_cache(
    redis_pool: &Option<RedisPool>,
    keys: &[&str],
    match_prefix: Option<&str>,
) {
    if let Some(mut conn) = get_redis_conn(redis_pool).await {
        for key in keys {
            let res: Result<(), _> = conn.del(*key).await;
            if let Err(err) = res {
                error!("Redis DEL error for key {}: {}", key, err);
            }
        }

        if let Some(prefix) = match_prefix {
            let pattern = format!("{}*", prefix);
            match redis::cmd("KEYS")
                .arg(&pattern)
                .query_async::<Vec<String>>(&mut conn)
                .await
            {
                Ok(matched_keys) if !matched_keys.is_empty() => {
                    let _: Result<(), _> = conn.del(&matched_keys).await;
                }
                _ => {}
            }
        }
    }
}
