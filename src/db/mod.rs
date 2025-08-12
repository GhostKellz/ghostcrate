use sqlx::{SqlitePool, Row};
use anyhow::Result;
use uuid::Uuid;
use chrono::Utc;
use crate::models::{User, Session, Crate, CrateVersion, PublishRequest};

mod organization_functions;
mod oidc_functions;
pub use organization_functions::*;
pub use oidc_functions::*;

pub async fn initialize_database(database_url: &str) -> Result<SqlitePool> {
    let pool = SqlitePool::connect(database_url).await?;
    
    // Create tables manually since we're not using migrations initially
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            email TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            is_admin BOOLEAN NOT NULL DEFAULT FALSE,
            github_id INTEGER,
            github_username TEXT,
            avatar_url TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        
        CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
        CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
        CREATE INDEX IF NOT EXISTS idx_users_github_id ON users(github_id);
        "#
    )
    .execute(&pool)
    .await?;
    
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            token TEXT UNIQUE NOT NULL,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
        );
        
        CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token);
        CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
        "#
    )
    .execute(&pool)
    .await?;

    // Create organizations table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS organizations (
            id TEXT PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            display_name TEXT NOT NULL,
            description TEXT,
            avatar_url TEXT,
            website TEXT,
            owner_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (owner_id) REFERENCES users (id) ON DELETE CASCADE
        );
        
        CREATE INDEX IF NOT EXISTS idx_organizations_name ON organizations(name);
        CREATE INDEX IF NOT EXISTS idx_organizations_owner_id ON organizations(owner_id);
        "#
    )
    .execute(&pool)
    .await?;

    // Create organization members table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS organization_members (
            id TEXT PRIMARY KEY,
            organization_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            role TEXT NOT NULL, -- 'owner', 'admin', 'member'
            invited_by TEXT,
            invited_at TEXT NOT NULL,
            joined_at TEXT,
            is_active BOOLEAN NOT NULL DEFAULT TRUE,
            FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE,
            FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
            FOREIGN KEY (invited_by) REFERENCES users (id) ON DELETE SET NULL,
            UNIQUE(organization_id, user_id)
        );
        
        CREATE INDEX IF NOT EXISTS idx_org_members_org_id ON organization_members(organization_id);
        CREATE INDEX IF NOT EXISTS idx_org_members_user_id ON organization_members(user_id);
        "#
    )
    .execute(&pool)
    .await?;

    // Create organization invites table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS organization_invites (
            id TEXT PRIMARY KEY,
            organization_id TEXT NOT NULL,
            email TEXT NOT NULL,
            role TEXT NOT NULL,
            invited_by TEXT NOT NULL,
            token TEXT UNIQUE NOT NULL,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            accepted_at TEXT,
            FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE,
            FOREIGN KEY (invited_by) REFERENCES users (id) ON DELETE CASCADE
        );
        
        CREATE INDEX IF NOT EXISTS idx_org_invites_token ON organization_invites(token);
        CREATE INDEX IF NOT EXISTS idx_org_invites_email ON organization_invites(email);
        CREATE INDEX IF NOT EXISTS idx_org_invites_org_id ON organization_invites(organization_id);
        "#
    )
    .execute(&pool)
    .await?;
    
    // Create crates table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS crates (
            id TEXT PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            description TEXT,
            homepage TEXT,
            documentation TEXT,
            repository TEXT,
            keywords TEXT, -- JSON encoded Vec<String>
            categories TEXT, -- JSON encoded Vec<String>
            license TEXT,
            owner_id TEXT NOT NULL,
            organization_id TEXT,
            downloads INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (owner_id) REFERENCES users (id) ON DELETE CASCADE,
            FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE SET NULL
        );
        
        CREATE INDEX IF NOT EXISTS idx_crates_name ON crates(name);
        CREATE INDEX IF NOT EXISTS idx_crates_owner_id ON crates(owner_id);
        CREATE INDEX IF NOT EXISTS idx_crates_organization_id ON crates(organization_id);
        CREATE INDEX IF NOT EXISTS idx_crates_downloads ON crates(downloads);
        "#
    )
    .execute(&pool)
    .await?;

    // Create download metrics table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS download_metrics (
            id TEXT PRIMARY KEY,
            crate_id TEXT NOT NULL,
            version TEXT NOT NULL,
            date TEXT NOT NULL, -- YYYY-MM-DD format
            count INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (crate_id) REFERENCES crates (id) ON DELETE CASCADE,
            UNIQUE(crate_id, version, date)
        );
        
        CREATE INDEX IF NOT EXISTS idx_download_metrics_crate_id ON download_metrics(crate_id);
        CREATE INDEX IF NOT EXISTS idx_download_metrics_date ON download_metrics(date);
        "#
    )
    .execute(&pool)
    .await?;
    
    // Create crate_versions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS crate_versions (
            id TEXT PRIMARY KEY,
            crate_id TEXT NOT NULL,
            version TEXT NOT NULL,
            checksum TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            dependencies TEXT, -- JSON encoded Vec<Dependency>
            features TEXT, -- JSON encoded HashMap<String, Vec<String>>
            yanked BOOLEAN NOT NULL DEFAULT FALSE,
            license TEXT,
            readme TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (crate_id) REFERENCES crates (id) ON DELETE CASCADE,
            UNIQUE(crate_id, version)
        );
        
        CREATE INDEX IF NOT EXISTS idx_crate_versions_crate_id ON crate_versions(crate_id);
        CREATE INDEX IF NOT EXISTS idx_crate_versions_version ON crate_versions(version);
        CREATE INDEX IF NOT EXISTS idx_crate_versions_yanked ON crate_versions(yanked);
        "#
    )
    .execute(&pool)
    .await?;
    
    Ok(pool)
}

pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    email: &str,
    password_hash: &str,
) -> Result<User> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, is_admin, github_id, github_username, avatar_url, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
    )
    .bind(id.to_string())
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .bind(false)
    .bind(None::<i64>)
    .bind(None::<String>)
    .bind(None::<String>)
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;
    
    let user = User {
        id,
        username: username.to_string(),
        email: email.to_string(),
        password_hash: password_hash.to_string(),
        is_admin: false,
        github_id: None,
        github_username: None,
        avatar_url: None,
        created_at: now,
        updated_at: now,
    };
    
    Ok(user)
}

pub async fn get_user_by_username(pool: &SqlitePool, username: &str) -> Result<Option<User>> {
    let row = sqlx::query(
        "SELECT id, username, email, password_hash, is_admin, github_id, github_username, avatar_url, created_at, updated_at FROM users WHERE username = ?1"
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    
    match row {
        Some(row) => {
            let user = User {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                username: row.get("username"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                is_admin: row.get("is_admin"),
                github_id: row.get("github_id"),
                github_username: row.get("github_username"),
                avatar_url: row.get("avatar_url"),
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?.with_timezone(&chrono::Utc),
            };
            Ok(Some(user))
        }
        None => Ok(None),
    }
}

pub async fn create_session(
    pool: &SqlitePool,
    user_id: Uuid,
    token: &str,
    expires_at: chrono::DateTime<Utc>,
) -> Result<Session> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    
    sqlx::query(
        "INSERT INTO sessions (id, user_id, token, expires_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5)"
    )
    .bind(id.to_string())
    .bind(user_id.to_string())
    .bind(token)
    .bind(expires_at.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;
    
    let session = Session {
        id,
        user_id,
        token: token.to_string(),
        expires_at,
        created_at: now,
    };
    
    Ok(session)
}

pub async fn get_session_by_token(pool: &SqlitePool, token: &str) -> Result<Option<Session>> {
    let row = sqlx::query(
        "SELECT id, user_id, token, expires_at, created_at FROM sessions WHERE token = ?1 AND expires_at > ?2"
    )
    .bind(token)
    .bind(Utc::now().to_rfc3339())
    .fetch_optional(pool)
    .await?;
    
    match row {
        Some(row) => {
            let session = Session {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                user_id: Uuid::parse_str(&row.get::<String, _>("user_id"))?,
                token: row.get("token"),
                expires_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("expires_at"))?.with_timezone(&chrono::Utc),
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&chrono::Utc),
            };
            Ok(Some(session))
        }
        None => Ok(None),
    }
}

pub async fn delete_session(pool: &SqlitePool, token: &str) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE token = ?1")
        .bind(token)
        .execute(pool)
        .await?;
    
    Ok(())
}

pub async fn get_user_by_id(pool: &SqlitePool, user_id: Uuid) -> Result<Option<User>> {
    let row = sqlx::query(
        "SELECT id, username, email, password_hash, is_admin, github_id, github_username, avatar_url, created_at, updated_at FROM users WHERE id = ?1"
    )
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await?;
    
    match row {
        Some(row) => {
            let user = User {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                username: row.get("username"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                is_admin: row.get("is_admin"),
                github_id: row.get("github_id"),
                github_username: row.get("github_username"),
                avatar_url: row.get("avatar_url"),
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?.with_timezone(&chrono::Utc),
            };
            Ok(Some(user))
        }
        None => Ok(None),
    }
}

pub async fn create_crate(
    pool: &SqlitePool,
    publish_req: &PublishRequest,
    owner_id: Uuid,
) -> Result<Crate> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    
    let keywords_json = serde_json::to_string(&publish_req.keywords)?;
    let categories_json = serde_json::to_string(&publish_req.categories)?;
    
    sqlx::query(
        r#"
        INSERT INTO crates (id, name, description, homepage, documentation, repository, keywords, categories, license, owner_id, downloads, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#
    )
    .bind(id.to_string())
    .bind(&publish_req.name)
    .bind(&publish_req.description)
    .bind(&publish_req.homepage)
    .bind(&publish_req.documentation)
    .bind(&publish_req.repository)
    .bind(&keywords_json)
    .bind(&categories_json)
    .bind(&publish_req.license)
    .bind(owner_id.to_string())
    .bind(0i64)
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;
    
    let crate_model = Crate {
        id,
        name: publish_req.name.clone(),
        description: publish_req.description.clone(),
        homepage: publish_req.homepage.clone(),
        documentation: publish_req.documentation.clone(),
        repository: publish_req.repository.clone(),
        keywords: Some(keywords_json),
        categories: Some(categories_json),
        license: publish_req.license.clone(),
        owner_id,
        downloads: 0,
        created_at: now,
        updated_at: now,
    };
    
    Ok(crate_model)
}

pub async fn get_crate_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Crate>> {
    let row = sqlx::query(
        "SELECT id, name, description, homepage, documentation, repository, keywords, categories, license, owner_id, downloads, created_at, updated_at FROM crates WHERE name = ?1"
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    
    match row {
        Some(row) => {
            let crate_model = Crate {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                name: row.get("name"),
                description: row.get("description"),
                homepage: row.get("homepage"),
                documentation: row.get("documentation"),
                repository: row.get("repository"),
                keywords: row.get("keywords"),
                categories: row.get("categories"),
                license: row.get("license"),
                owner_id: Uuid::parse_str(&row.get::<String, _>("owner_id"))?,
                downloads: row.get("downloads"),
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?.with_timezone(&chrono::Utc),
            };
            Ok(Some(crate_model))
        }
        None => Ok(None),
    }
}

pub async fn create_crate_version(
    pool: &SqlitePool,
    crate_id: Uuid,
    publish_req: &PublishRequest,
    checksum: &str,
    file_size: i64,
) -> Result<CrateVersion> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    
    let dependencies_json = serde_json::to_string(&publish_req.deps)?;
    let features_json = serde_json::to_string(&publish_req.features)?;
    
    sqlx::query(
        r#"
        INSERT INTO crate_versions (id, crate_id, version, checksum, file_size, dependencies, features, yanked, license, readme, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#
    )
    .bind(id.to_string())
    .bind(crate_id.to_string())
    .bind(&publish_req.vers)
    .bind(checksum)
    .bind(file_size)
    .bind(&dependencies_json)
    .bind(&features_json)
    .bind(false)
    .bind(&publish_req.license)
    .bind(&publish_req.readme)
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;
    
    let version = CrateVersion {
        id,
        crate_id,
        version: publish_req.vers.clone(),
        checksum: checksum.to_string(),
        file_size,
        dependencies: Some(dependencies_json),
        features: Some(features_json),
        yanked: false,
        license: publish_req.license.clone(),
        readme: publish_req.readme.clone(),
        created_at: now,
    };
    
    Ok(version)
}

pub async fn get_crate_versions(pool: &SqlitePool, crate_id: Uuid) -> Result<Vec<CrateVersion>> {
    let rows = sqlx::query(
        "SELECT id, crate_id, version, checksum, file_size, dependencies, features, yanked, license, readme, created_at FROM crate_versions WHERE crate_id = ?1 ORDER BY created_at DESC"
    )
    .bind(crate_id.to_string())
    .fetch_all(pool)
    .await?;
    
    let mut versions = Vec::new();
    for row in rows {
        let version = CrateVersion {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            crate_id: Uuid::parse_str(&row.get::<String, _>("crate_id"))?,
            version: row.get("version"),
            checksum: row.get("checksum"),
            file_size: row.get("file_size"),
            dependencies: row.get("dependencies"),
            features: row.get("features"),
            yanked: row.get("yanked"),
            license: row.get("license"),
            readme: row.get("readme"),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&chrono::Utc),
        };
        versions.push(version);
    }
    
    Ok(versions)
}

pub async fn increment_download_count(pool: &SqlitePool, crate_id: Uuid) -> Result<()> {
    sqlx::query("UPDATE crates SET downloads = downloads + 1 WHERE id = ?1")
        .bind(crate_id.to_string())
        .execute(pool)
        .await?;
    
    Ok(())
}

pub async fn search_crates(
    pool: &SqlitePool,
    query: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<Crate>> {
    let search_pattern = format!("%{}%", query);
    
    let rows = sqlx::query(
        r#"
        SELECT id, name, description, homepage, documentation, repository, keywords, categories, license, owner_id, downloads, created_at, updated_at 
        FROM crates 
        WHERE name LIKE ?1 OR description LIKE ?1 
        ORDER BY downloads DESC, name ASC 
        LIMIT ?2 OFFSET ?3
        "#
    )
    .bind(&search_pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    
    let mut crates = Vec::new();
    for row in rows {
        let crate_model = Crate {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            name: row.get("name"),
            description: row.get("description"),
            homepage: row.get("homepage"),
            documentation: row.get("documentation"),
            repository: row.get("repository"),
            keywords: row.get("keywords"),
            categories: row.get("categories"),
            license: row.get("license"),
            owner_id: Uuid::parse_str(&row.get::<String, _>("owner_id"))?,
            downloads: row.get("downloads"),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?.with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?.with_timezone(&chrono::Utc),
        };
        crates.push(crate_model);
    }
    
    Ok(crates)
}

pub async fn count_search_results(pool: &SqlitePool, query: &str) -> Result<i64> {
    let search_pattern = format!("%{}%", query);
    
    let row = sqlx::query(
        "SELECT COUNT(*) as count FROM crates WHERE name LIKE ?1 OR description LIKE ?1"
    )
    .bind(&search_pattern)
    .fetch_one(pool)
    .await?;
    
    Ok(row.get("count"))
}

// Health check functions
pub async fn count_total_crates(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM crates")
        .fetch_one(pool)
        .await?;
    
    Ok(row.get("count"))
}

pub async fn count_total_versions(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM crate_versions")
        .fetch_one(pool)
        .await?;
    
    Ok(row.get("count"))
}

pub async fn count_total_downloads(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COALESCE(SUM(downloads), 0) as total FROM crates")
        .fetch_one(pool)
        .await?;
    
    Ok(row.get("total"))
}

pub async fn count_total_users(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM users")
        .fetch_one(pool)
        .await?;
    
    Ok(row.get("count"))
}

// Additional helper functions for health stats
pub async fn count_total_organizations(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM organizations")
        .fetch_one(pool)
        .await?;
    
    Ok(row.get("count"))
}

pub async fn count_downloads_last_days(pool: &SqlitePool, days: i64) -> Result<i64> {
    let cutoff_date = (chrono::Utc::now() - chrono::Duration::days(days)).format("%Y-%m-%d").to_string();
    
    let row = sqlx::query("SELECT COALESCE(SUM(count), 0) as total FROM download_metrics WHERE date >= ?1")
        .bind(cutoff_date)
        .fetch_one(pool)
        .await?;
    
    Ok(row.get("total"))
}

pub async fn count_new_crates_last_days(pool: &SqlitePool, days: i64) -> Result<i64> {
    let cutoff_date = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();
    
    let row = sqlx::query("SELECT COUNT(*) as count FROM crates WHERE created_at >= ?1")
        .bind(cutoff_date)
        .fetch_one(pool)
        .await?;
    
    Ok(row.get("count"))
}

pub async fn count_new_users_last_days(pool: &SqlitePool, days: i64) -> Result<i64> {
    let cutoff_date = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();
    
    let row = sqlx::query("SELECT COUNT(*) as count FROM users WHERE created_at >= ?1")
        .bind(cutoff_date)
        .fetch_one(pool)
        .await?;
    
    Ok(row.get("count"))
}

pub async fn get_top_crates(pool: &SqlitePool, limit: i64) -> Result<Vec<crate::models::TopCrateStats>> {
    let rows = sqlx::query(
        r#"
        SELECT c.name, c.downloads, c.description,
               COALESCE(
                   (SELECT cv.version FROM crate_versions cv 
                    WHERE cv.crate_id = c.id 
                    ORDER BY cv.created_at DESC LIMIT 1), 
                   '0.0.0'
               ) as latest_version
        FROM crates c 
        ORDER BY c.downloads DESC 
        LIMIT ?1
        "#
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    
    let mut stats = Vec::new();
    for row in rows {
        // For now, just set downloads_last_30_days to 0 as we'd need more complex query
        stats.push(crate::models::TopCrateStats {
            name: row.get("name"),
            total_downloads: row.get("downloads"),
            downloads_last_30_days: 0, // TODO: implement proper calculation
            latest_version: row.get("latest_version"),
            description: row.get("description"),
        });
    }
    
    Ok(stats)
}