//! Persistence layer for connections.
//!
//! All CRUD operations on the `connections` table live here.
//! The module is intentionally thin — business logic (adapter building,
//! health checks) belongs in [`crate::connections`].

use rusqlite::Connection;

use super::{ConnectionInfo, ConnectionRow, HealthInfo, ProductType};

/// List all connections, returning lightweight info (token redacted).
pub fn list(conn: &Connection) -> Result<Vec<ConnectionInfo>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, product_type, url, auth_type, auth_token,
                    metadata, status, is_primary, created_at, last_tested_at
               FROM connections
              ORDER BY is_primary DESC, created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ConnectionRow {
                id: row.get(0)?,
                name: row.get(1)?,
                product_type: parse_product_type(row.get::<_, String>(2)?),
                url: row.get(3)?,
                auth_type: row.get(4)?,
                auth_token: row.get(5)?,
                metadata: row
                    .get::<_, Option<String>>(6)?
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(serde_json::json!({})),
                status: row.get(7)?,
                is_primary: row.get::<_, i32>(8)? != 0,
                created_at: row.get(9)?,
                last_tested_at: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .map(|r| ConnectionInfo::from(&r))
        .collect();

    Ok(rows)
}

/// Get a single connection by id (full row, token included).
pub fn get(conn: &Connection, id: &str) -> Result<ConnectionRow, String> {
    conn.query_row(
        "SELECT id, name, product_type, url, auth_type, auth_token,
                metadata, status, is_primary, created_at, last_tested_at
           FROM connections WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(ConnectionRow {
                id: row.get(0)?,
                name: row.get(1)?,
                product_type: parse_product_type(row.get::<_, String>(2)?),
                url: row.get(3)?,
                auth_type: row.get(4)?,
                auth_token: row.get(5)?,
                metadata: row
                    .get::<_, Option<String>>(6)?
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(serde_json::json!({})),
                status: row.get(7)?,
                is_primary: row.get::<_, i32>(8)? != 0,
                created_at: row.get(9)?,
                last_tested_at: row.get(10)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

/// Insert a new connection.  Returns the new id.
#[allow(clippy::too_many_arguments)]
pub fn insert(
    conn: &mut Connection,
    id: &str,
    name: &str,
    product_type: ProductType,
    url: &str,
    auth_type: Option<&str>,
    auth_token: Option<&str>,
    metadata: Option<&serde_json::Value>,
) -> Result<(), String> {
    let meta_json = metadata
        .map(|m| serde_json::to_string(m).unwrap_or_else(|_| "{}".into()))
        .unwrap_or_else(|| "{}".into());
    conn.execute(
        "INSERT INTO connections (id, name, product_type, url, auth_type, auth_token, metadata, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'unknown', datetime('now'))",
        rusqlite::params![id, name, product_type.to_string(), url, auth_type, auth_token, meta_json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Update fields on an existing connection.
pub fn update(
    conn: &mut Connection,
    id: &str,
    name: Option<&str>,
    url: Option<&str>,
    auth_type: Option<&str>,
    auth_token: Option<&str>,
    metadata: Option<&serde_json::Value>,
) -> Result<(), String> {
    let mut sets = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(v) = name {
        sets.push("name = ?");
        params.push(Box::new(v.to_string()));
    }
    if let Some(v) = url {
        sets.push("url = ?");
        params.push(Box::new(v.to_string()));
    }
    if let Some(v) = auth_type {
        sets.push("auth_type = ?");
        params.push(Box::new(v.to_string()));
    }
    if let Some(v) = auth_token {
        sets.push("auth_token = ?");
        params.push(Box::new(v.to_string()));
    }
    if let Some(v) = metadata {
        sets.push("metadata = ?");
        params.push(Box::new(
            serde_json::to_string(v).unwrap_or_else(|_| "{}".into()),
        ));
    }
    if sets.is_empty() {
        return Ok(());
    }
    params.push(Box::new(id.to_string()));

    let sql = format!("UPDATE connections SET {} WHERE id = ?", sets.join(", "));
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    conn.execute(&sql, param_refs.as_slice())
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a connection by id.
pub fn delete(conn: &mut Connection, id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM connections WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Set a connection as the primary.  Unsets all others of the same product type.
pub fn set_primary(conn: &mut Connection, id: &str) -> Result<(), String> {
    let row = conn
        .query_row(
            "SELECT product_type FROM connections WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE connections SET is_primary = 0 WHERE product_type = ?1",
        rusqlite::params![row],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE connections SET is_primary = 1 WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Get the primary connection for a product type (full row, token included).
pub fn get_primary(
    conn: &Connection,
    product_type: ProductType,
) -> Result<Option<ConnectionRow>, String> {
    let result = conn.query_row(
        "SELECT id, name, product_type, url, auth_type, auth_token,
                metadata, status, is_primary, created_at, last_tested_at
           FROM connections
          WHERE product_type = ?1 AND is_primary = 1
          LIMIT 1",
        rusqlite::params![product_type.to_string()],
        |row| {
            Ok(ConnectionRow {
                id: row.get(0)?,
                name: row.get(1)?,
                product_type: parse_product_type(row.get::<_, String>(2)?),
                url: row.get(3)?,
                auth_type: row.get(4)?,
                auth_token: row.get(5)?,
                metadata: row
                    .get::<_, Option<String>>(6)?
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(serde_json::json!({})),
                status: row.get(7)?,
                is_primary: row.get::<_, i32>(8)? != 0,
                created_at: row.get(9)?,
                last_tested_at: row.get(10)?,
            })
        },
    );
    match result {
        Ok(row) => Ok(Some(row)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Update the status and last_tested_at of a connection.
pub fn update_status(
    conn: &mut Connection,
    id: &str,
    status: &str,
    _health: &HealthInfo,
) -> Result<(), String> {
    conn.execute(
        "UPDATE connections SET status = ?1, last_tested_at = datetime('now')
         WHERE id = ?2",
        rusqlite::params![status, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Check if the connections table has any rows.
pub fn is_empty(conn: &Connection) -> Result<bool, String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM connections", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    Ok(count == 0)
}

/// Seed a default local uteke connection.  Called once on first boot when
/// the connections table is empty.
pub fn seed_default(conn: &mut Connection, detected_url: &str) -> Result<String, String> {
    let id = nanoid::nanoid!(12);
    insert(
        conn,
        &id,
        "Local uteke",
        ProductType::Uteke,
        detected_url,
        None,
        None,
        None,
    )?;
    set_primary(conn, &id)?;
    Ok(id)
}

// ─── Helpers ────────────────────────────────────────────────────────

fn parse_product_type(s: String) -> ProductType {
    match s.as_str() {
        "uteke" => ProductType::Uteke,
        other => {
            eprintln!("Unknown product_type '{other}', defaulting to Uteke");
            ProductType::Uteke
        }
    }
}
