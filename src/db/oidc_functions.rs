use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use anyhow::Result;
use chrono::Utc;

use crate::models::{User, OidcUserLink};

/// Get user by OIDC external link
pub async fn get_user_by_oidc_link(
    pool: &SqlitePool,
    external_id: &str,
    provider: &str,
) -> Result<Option<User>> {
    let query = r#"
        SELECT u.* FROM users u
        JOIN oidc_user_links oul ON u.id = oul.user_id
        WHERE oul.external_id = ? AND oul.provider_type = ?
    "#;

    let row = sqlx::query(query)
        .bind(external_id)
        .bind(provider)
        .fetch_optional(pool)
        .await?;

    if let Some(row) = row {
        let user = User {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            is_admin: row.get("is_admin"),
            github_id: row.get("github_id"),
            github_username: row.get("github_username"),
            avatar_url: row.get("avatar_url"),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?
                .with_timezone(&chrono::Utc),
        };
        Ok(Some(user))
    } else {
        Ok(None)
    }
}

/// Get user by email
pub async fn get_user_by_email(pool: &SqlitePool, email: &str) -> Result<Option<User>> {
    let query = "SELECT * FROM users WHERE email = ?";

    let row = sqlx::query(query)
        .bind(email)
        .fetch_optional(pool)
        .await?;

    if let Some(row) = row {
        let user = User {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            is_admin: row.get("is_admin"),
            github_id: row.get("github_id"),
            github_username: row.get("github_username"),
            avatar_url: row.get("avatar_url"),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?
                .with_timezone(&chrono::Utc),
        };
        Ok(Some(user))
    } else {
        Ok(None)
    }
}

/// Create OIDC user (user created via OIDC authentication)
pub async fn create_oidc_user(pool: &SqlitePool, user: &User) -> Result<()> {
    let query = r#"
        INSERT INTO users (id, username, email, password_hash, is_admin, github_id, github_username, avatar_url, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#;

    sqlx::query(query)
        .bind(user.id.to_string())
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(user.is_admin)
        .bind(user.github_id)
        .bind(&user.github_username)
        .bind(&user.avatar_url)
        .bind(user.created_at.to_rfc3339())
        .bind(user.updated_at.to_rfc3339())
        .execute(pool)
        .await?;

    Ok(())
}

/// Create OIDC user link
pub async fn create_oidc_user_link(
    pool: &SqlitePool,
    user_id: Uuid,
    external_id: &str,
    provider_type: &str,
    email: &str,
    name: Option<&str>,
) -> Result<()> {
    let query = r#"
        INSERT INTO oidc_user_links (id, user_id, external_id, provider_type, email, name, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    "#;

    let now = Utc::now();
    
    sqlx::query(query)
        .bind(Uuid::new_v4().to_string())
        .bind(user_id.to_string())
        .bind(external_id)
        .bind(provider_type)
        .bind(email)
        .bind(name)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(pool)
        .await?;

    Ok(())
}

/// Update OIDC user link last login
pub async fn update_oidc_user_link_last_login(
    pool: &SqlitePool,
    user_id: Uuid,
    provider_type: &str,
) -> Result<()> {
    let query = r#"
        UPDATE oidc_user_links 
        SET last_login = ?, updated_at = ?
        WHERE user_id = ? AND provider_type = ?
    "#;

    let now = Utc::now();
    
    sqlx::query(query)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(user_id.to_string())
        .bind(provider_type)
        .execute(pool)
        .await?;

    Ok(())
}

/// Get OIDC links for a user
pub async fn get_user_oidc_links(pool: &SqlitePool, user_id: Uuid) -> Result<Vec<OidcUserLink>> {
    let query = "SELECT * FROM oidc_user_links WHERE user_id = ?";

    let rows = sqlx::query(query)
        .bind(user_id.to_string())
        .fetch_all(pool)
        .await?;

    let mut links = Vec::new();
    for row in rows {
        let link = OidcUserLink {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            user_id: Uuid::parse_str(&row.get::<String, _>("user_id"))?,
            provider_id: Uuid::parse_str(&row.get::<String, _>("provider_id"))?, // This will need to be nullable in the actual table
            external_id: row.get("external_id"),
            email: row.get("email"),
            name: row.get("name"),
            avatar_url: row.get("avatar_url"),
            last_login: row.get::<Option<String>, _>("last_login")
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?
                .with_timezone(&chrono::Utc),
        };
        links.push(link);
    }

    Ok(links)
}
