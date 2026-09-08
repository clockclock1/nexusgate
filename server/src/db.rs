use crate::config::ServerConfig;
use anyhow::Context;
use p2p_security::{generate_node_token, hash_password, hash_token};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;

pub async fn init_db(config: &ServerConfig) -> anyhow::Result<SqlitePool> {
    config.ensure_db_parent()?;
    let url = &config.database_url;
    let options = SqliteConnectOptions::from_str(url)
        .with_context(|| format!("invalid database url: {url}"))?
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .context("connect sqlite")?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            email TEXT,
            created_at TEXT NOT NULL,
            last_login TEXT
        );
        CREATE TABLE IF NOT EXISTS nodes (
            node_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            token_hash TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'offline',
            public_ip TEXT,
            nat_type TEXT,
            version TEXT NOT NULL DEFAULT '0.1.0',
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS services (
            service_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            node_id TEXT NOT NULL,
            protocol TEXT NOT NULL,
            local_addr TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            description TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS routes (
            route_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            public_port INTEGER NOT NULL,
            protocol TEXT NOT NULL,
            node_id TEXT NOT NULL,
            service_id TEXT NOT NULL,
            host TEXT,
            enabled INTEGER NOT NULL DEFAULT 1,
            description TEXT,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS audit_logs (
            id TEXT PRIMARY KEY,
            actor TEXT,
            action TEXT NOT NULL,
            detail TEXT,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .execute(&pool)
    .await
    .context("migrate schema")?;

    seed_admin(&pool, config).await?;
    Ok(pool)
}

async fn seed_admin(pool: &SqlitePool, config: &ServerConfig) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if count == 0 {
        let id = uuid::Uuid::new_v4().to_string();
        let hash = hash_password(&config.admin_password)?;
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO users (id, username, password_hash, role, enabled, created_at) VALUES (?, ?, ?, 'admin', 1, ?)",
        )
        .bind(&id)
        .bind(&config.admin_user)
        .bind(&hash)
        .bind(&now)
        .execute(pool)
        .await?;
        tracing::info!(user = %config.admin_user, "seeded default admin user");
    }
    Ok(())
}

pub async fn create_node(
    pool: &SqlitePool,
    name: &str,
    node_id: Option<String>,
) -> anyhow::Result<(String, String)> {
    let node_id = node_id.unwrap_or_else(|| format!("edge-{}", &uuid::Uuid::new_v4().to_string()[..8]));
    let token = generate_node_token();
    let token_hash = hash_token(&token);
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO nodes (node_id, name, token_hash, status, version, enabled, created_at) VALUES (?, ?, ?, 'offline', '0.1.0', 1, ?)",
    )
    .bind(&node_id)
    .bind(name)
    .bind(&token_hash)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok((node_id, token))
}

pub async fn get_node_token_hash(pool: &SqlitePool, node_id: &str) -> anyhow::Result<Option<(String, bool, String)>> {
    let row = sqlx::query("SELECT token_hash, enabled, name FROM nodes WHERE node_id = ?")
        .bind(node_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| {
        (
            r.get::<String, _>("token_hash"),
            r.get::<i64, _>("enabled") == 1,
            r.get::<String, _>("name"),
        )
    }))
}

pub async fn list_nodes(pool: &SqlitePool) -> anyhow::Result<Vec<sqlx::sqlite::SqliteRow>> {
    let rows = sqlx::query("SELECT * FROM nodes ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn audit(pool: &SqlitePool, actor: &str, action: &str, detail: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO audit_logs (id, actor, action, detail, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(actor)
    .bind(action)
    .bind(detail)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn load_routes_into(pool: &SqlitePool, table: &p2p_router::RouteTable) -> anyhow::Result<()> {
    let rows = sqlx::query("SELECT * FROM routes WHERE enabled = 1")
        .fetch_all(pool)
        .await?;
    for r in rows {
        let service_id: String = r.get("service_id");
        let local_addr: String = sqlx::query_scalar("SELECT local_addr FROM services WHERE service_id = ?")
            .bind(&service_id)
            .fetch_optional(pool)
            .await?
            .unwrap_or_else(|| "127.0.0.1:0".into());
        let rule = p2p_common::RouteRule {
            id: r.get("route_id"),
            protocol: p2p_common::ProtocolKind::parse(&r.get::<String, _>("protocol"))
                .unwrap_or(p2p_common::ProtocolKind::Tcp),
            listen_port: Some(r.get::<i64, _>("public_port") as u16),
            host: r.get("host"),
            sni: None,
            node_id: p2p_common::NodeId::new(r.get::<String, _>("node_id")),
            service_id,
            local_addr,
            enabled: true,
            priority: 0,
        };
        table.upsert(rule);
    }
    Ok(())
}
