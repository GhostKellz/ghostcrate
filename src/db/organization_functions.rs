// Organization database functions for db/mod.rs

use crate::models::{
    user::User,
    organization::{Organization, OrganizationMember, OrganizationRole, OrganizationInvite},
    organization::{CreateOrganizationRequest, UpdateOrganizationRequest},
    metrics::TopCrateStats,
};
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::Result;

// Organization functions
pub async fn organization_exists(pool: &SqlitePool, name: &str) -> Result<bool> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organizations WHERE name = ?1")
        .bind(name)
        .fetch_one(pool)
        .await?;
    Ok(count > 0)
}

pub async fn create_organization(
    pool: &SqlitePool,
    request: &CreateOrganizationRequest,
    owner_id: Uuid,
) -> Result<Organization> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO organizations (id, name, display_name, description, website, owner_id, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#
    )
    .bind(id.to_string())
    .bind(&request.name)
    .bind(&request.display_name)
    .bind(&request.description)
    .bind(&request.website)
    .bind(owner_id.to_string())
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;

    // Add owner as organization member
    let member_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO organization_members (id, organization_id, user_id, role, invited_by, invited_at, joined_at, is_active)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#
    )
    .bind(member_id.to_string())
    .bind(id.to_string())
    .bind(owner_id.to_string())
    .bind("owner")
    .bind(owner_id.to_string())
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .bind(true)
    .execute(pool)
    .await?;

    Ok(Organization {
        id,
        name: request.name.clone(),
        display_name: request.display_name.clone(),
        description: request.description.clone(),
        avatar_url: None,
        website: request.website.clone(),
        owner_id,
        created_at: now,
        updated_at: now,
    })
}

pub async fn get_organization_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Organization>> {
    let row = sqlx::query(
        "SELECT id, name, display_name, description, avatar_url, website, owner_id, created_at, updated_at FROM organizations WHERE name = ?1"
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        Ok(Some(Organization {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            name: row.get("name"),
            display_name: row.get("display_name"),
            description: row.get("description"),
            avatar_url: row.get("avatar_url"),
            website: row.get("website"),
            owner_id: Uuid::parse_str(&row.get::<String, _>("owner_id"))?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at")).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at")).unwrap().with_timezone(&chrono::Utc),
        }))
    } else {
        Ok(None)
    }
}

pub async fn get_organization_by_id(pool: &SqlitePool, org_id: Uuid) -> Result<Option<Organization>> {
    let row = sqlx::query(
        "SELECT id, name, display_name, description, avatar_url, website, owner_id, created_at, updated_at FROM organizations WHERE id = ?1"
    )
    .bind(org_id.to_string())
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        Ok(Some(Organization {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            name: row.get("name"),
            display_name: row.get("display_name"),
            description: row.get("description"),
            avatar_url: row.get("avatar_url"),
            website: row.get("website"),
            owner_id: Uuid::parse_str(&row.get::<String, _>("owner_id"))?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at")).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at")).unwrap().with_timezone(&chrono::Utc),
        }))
    } else {
        Ok(None)
    }
}

pub async fn list_user_organizations(pool: &SqlitePool, user_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Organization>> {
    let rows = sqlx::query(
        r#"
        SELECT o.id, o.name, o.display_name, o.description, o.avatar_url, o.website, o.owner_id, o.created_at, o.updated_at
        FROM organizations o
        JOIN organization_members om ON o.id = om.organization_id
        WHERE om.user_id = ?1 AND om.is_active = true
        ORDER BY o.name ASC
        LIMIT ?2 OFFSET ?3
        "#
    )
    .bind(user_id.to_string())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let mut organizations = Vec::new();
    for row in rows {
        organizations.push(Organization {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            name: row.get("name"),
            display_name: row.get("display_name"),
            description: row.get("description"),
            avatar_url: row.get("avatar_url"),
            website: row.get("website"),
            owner_id: Uuid::parse_str(&row.get::<String, _>("owner_id"))?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at")).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at")).unwrap().with_timezone(&chrono::Utc),
        });
    }

    Ok(organizations)
}

pub async fn list_user_organization_invites(pool: &SqlitePool, user_email: &str) -> Result<Vec<OrganizationInvite>> {
    let rows = sqlx::query(
        "SELECT id, organization_id, email, role, invited_by, token, expires_at, created_at, accepted_at FROM organization_invites WHERE email = ?1 AND expires_at > ?2 AND accepted_at IS NULL"
    )
    .bind(user_email)
    .bind(Utc::now().to_rfc3339())
    .fetch_all(pool)
    .await?;

    let mut invites = Vec::new();
    for row in rows {
        let role = match row.get::<String, _>("role").as_str() {
            "owner" => OrganizationRole::Owner,
            "admin" => OrganizationRole::Admin,
            _ => OrganizationRole::Member,
        };

        invites.push(OrganizationInvite {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            organization_id: Uuid::parse_str(&row.get::<String, _>("organization_id"))?,
            email: row.get("email"),
            role,
            invited_by: Uuid::parse_str(&row.get::<String, _>("invited_by"))?,
            token: row.get("token"),
            expires_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("expires_at")).unwrap().with_timezone(&chrono::Utc),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at")).unwrap().with_timezone(&chrono::Utc),
            accepted_at: row.get::<Option<String>, _>("accepted_at").map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
        });
    }

    Ok(invites)
}

pub async fn update_organization(
    pool: &SqlitePool,
    org_id: Uuid,
    request: &UpdateOrganizationRequest,
) -> Result<Organization> {
    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE organizations 
        SET display_name = COALESCE(?1, display_name),
            description = COALESCE(?2, description),
            website = COALESCE(?3, website),
            avatar_url = COALESCE(?4, avatar_url),
            updated_at = ?5
        WHERE id = ?6
        "#
    )
    .bind(&request.display_name)
    .bind(&request.description)
    .bind(&request.website)
    .bind(&request.avatar_url)
    .bind(now.to_rfc3339())
    .bind(org_id.to_string())
    .execute(pool)
    .await?;

    // Fetch and return updated organization
    let row = sqlx::query(
        "SELECT id, name, display_name, description, avatar_url, website, owner_id, created_at, updated_at FROM organizations WHERE id = ?1"
    )
    .bind(org_id.to_string())
    .fetch_one(pool)
    .await?;

    Ok(Organization {
        id: Uuid::parse_str(&row.get::<String, _>("id"))?,
        name: row.get("name"),
        display_name: row.get("display_name"),
        description: row.get("description"),
        avatar_url: row.get("avatar_url"),
        website: row.get("website"),
        owner_id: Uuid::parse_str(&row.get::<String, _>("owner_id"))?,
        created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at")).unwrap().with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at")).unwrap().with_timezone(&chrono::Utc),
    })
}

pub async fn delete_organization(pool: &SqlitePool, org_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM organizations WHERE id = ?1")
        .bind(org_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_organization_member_count(pool: &SqlitePool, org_id: Uuid) -> Result<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM organization_members WHERE organization_id = ?1 AND is_active = true"
    )
    .bind(org_id.to_string())
    .fetch_one(pool)
    .await?;
    Ok(count)
}

pub async fn get_organization_crate_count(pool: &SqlitePool, org_id: Uuid) -> Result<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM crates WHERE organization_id = ?1"
    )
    .bind(org_id.to_string())
    .fetch_one(pool)
    .await?;
    Ok(count)
}

// Statistics functions
pub async fn count_total_crates(pool: &SqlitePool) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM crates")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

pub async fn count_total_versions(pool: &SqlitePool) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM crate_versions")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

pub async fn count_total_downloads(pool: &SqlitePool) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(downloads), 0) FROM crates")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

pub async fn count_total_users(pool: &SqlitePool) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

pub async fn count_total_organizations(pool: &SqlitePool) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organizations")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

pub async fn count_downloads_last_days(pool: &SqlitePool, days: i32) -> Result<i64> {
    // This would need a downloads tracking table
    // For now, return 0
    Ok(0)
}

pub async fn count_new_crates_last_days(pool: &SqlitePool, days: i32) -> Result<i64> {
    let since = Utc::now() - chrono::Duration::days(days as i64);
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM crates WHERE created_at >= ?1"
    )
    .bind(since.to_rfc3339())
    .fetch_one(pool)
    .await?;
    Ok(count)
}

pub async fn count_new_users_last_days(pool: &SqlitePool, days: i32) -> Result<i64> {
    let since = Utc::now() - chrono::Duration::days(days as i64);
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE created_at >= ?1"
    )
    .bind(since.to_rfc3339())
    .fetch_one(pool)
    .await?;
    Ok(count)
}

pub async fn get_top_crates(pool: &SqlitePool, limit: i32) -> Result<Vec<TopCrateStats>> {
    let rows = sqlx::query(
        r#"
        SELECT c.name, c.description, c.downloads, 
               COALESCE(cv.version, '0.0.0') as latest_version
        FROM crates c
        LEFT JOIN crate_versions cv ON c.id = cv.crate_id
        GROUP BY c.id
        ORDER BY c.downloads DESC
        LIMIT ?1
        "#
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut top_crates = Vec::new();
    for row in rows {
        top_crates.push(TopCrateStats {
            name: row.get("name"),
            total_downloads: row.get("downloads"),
            downloads_last_30_days: 0, // Would need tracking
            latest_version: row.get("latest_version"),
            description: row.get("description"),
        });
    }

    Ok(top_crates)
}

// GitHub-related functions
pub async fn get_user_by_github_id(pool: &SqlitePool, github_id: i64) -> Result<Option<User>> {
    let row = sqlx::query(
        "SELECT id, username, email, password_hash, is_admin, github_id, github_username, avatar_url, created_at, updated_at FROM users WHERE github_id = ?1"
    )
    .bind(github_id)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        Ok(Some(User {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            is_admin: row.get("is_admin"),
            github_id: row.get("github_id"),
            github_username: row.get("github_username"),
            avatar_url: row.get("avatar_url"),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at")).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at")).unwrap().with_timezone(&chrono::Utc),
        }))
    } else {
        Ok(None)
    }
}

pub async fn create_github_user(
    pool: &SqlitePool,
    username: &str,
    email: &str,
    github_id: i64,
    name: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<User> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    
    // Create a dummy password hash since this is a GitHub user
    let password_hash = format!("github_{}", github_id);
    
    sqlx::query(
        r#"
        INSERT INTO users (id, username, email, password_hash, is_admin, github_id, github_username, avatar_url, created_at, updated_at) 
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#
    )
    .bind(id.to_string())
    .bind(username)
    .bind(email)
    .bind(&password_hash)
    .bind(false)
    .bind(github_id)
    .bind(username)
    .bind(avatar_url)
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;
    
    Ok(User {
        id,
        username: username.to_string(),
        email: email.to_string(),
        password_hash,
        is_admin: false,
        github_id: Some(github_id),
        github_username: Some(username.to_string()),
        avatar_url: avatar_url.map(|s| s.to_string()),
        created_at: now,
        updated_at: now,
    })
}

pub async fn disconnect_github_user(pool: &SqlitePool, user_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE users SET github_id = NULL, github_username = NULL WHERE id = ?1"
    )
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn link_github_user(
    pool: &SqlitePool,
    user_id: Uuid,
    github_id: i64,
    name: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<User> {
    let now = Utc::now();
    
    sqlx::query(
        r#"
        UPDATE users 
        SET github_id = ?1, github_username = ?2, avatar_url = ?3, updated_at = ?4 
        WHERE id = ?5
        "#
    )
    .bind(github_id)
    .bind(name)
    .bind(avatar_url)
    .bind(now.to_rfc3339())
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    
    super::get_user_by_id(pool, user_id).await?.ok_or_else(|| anyhow::anyhow!("User not found"))
}

// Organization membership functions
pub async fn user_can_manage_organization(pool: &SqlitePool, user_id: Uuid, org_id: Uuid) -> Result<bool> {
    let role = get_user_organization_role(pool, user_id, org_id).await?;
    match role {
        Some(OrganizationRole::Owner) | Some(OrganizationRole::Admin) => Ok(true),
        _ => Ok(false),
    }
}

pub async fn get_organization_members(
    pool: &SqlitePool, 
    org_id: Uuid, 
    limit: i64, 
    offset: i64
) -> Result<Vec<(OrganizationMember, User)>> {
    let rows = sqlx::query(
        r#"
        SELECT 
            om.id, om.organization_id, om.user_id, om.role, om.invited_by, om.invited_at, om.joined_at, om.is_active,
            u.id as user_id, u.username, u.email, u.password_hash, u.is_admin, u.github_id, u.github_username, u.avatar_url, u.created_at as user_created_at, u.updated_at as user_updated_at
        FROM organization_members om
        JOIN users u ON om.user_id = u.id
        WHERE om.organization_id = ?1 AND om.is_active = true
        ORDER BY om.joined_at DESC
        LIMIT ?2 OFFSET ?3
        "#
    )
    .bind(org_id.to_string())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let mut members = Vec::new();
    for row in rows {
        let member = OrganizationMember {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            organization_id: Uuid::parse_str(&row.get::<String, _>("organization_id"))?,
            user_id: Uuid::parse_str(&row.get::<String, _>("user_id"))?,
            role: match row.get::<String, _>("role").as_str() {
                "owner" => OrganizationRole::Owner,
                "admin" => OrganizationRole::Admin,
                "viewer" => OrganizationRole::Viewer,
                _ => OrganizationRole::Member,
            },
            invited_by: row.get::<Option<String>, _>("invited_by").map(|s| Uuid::parse_str(&s)).transpose()?,
            invited_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("invited_at")).unwrap().with_timezone(&chrono::Utc),
            joined_at: row.get::<Option<String>, _>("joined_at").map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            is_active: row.get("is_active"),
        };

        let user = User {
            id: Uuid::parse_str(&row.get::<String, _>("user_id"))?,
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            is_admin: row.get("is_admin"),
            github_id: row.get("github_id"),
            github_username: row.get("github_username"),
            avatar_url: row.get("avatar_url"),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("user_created_at")).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("user_updated_at")).unwrap().with_timezone(&chrono::Utc),
        };

        members.push((member, user));
    }

    Ok(members)
}

pub async fn get_user_organization_role(
    pool: &SqlitePool, 
    user_id: Uuid, 
    org_id: Uuid
) -> Result<Option<OrganizationRole>> {
    let row = sqlx::query(
        "SELECT role FROM organization_members WHERE user_id = ?1 AND organization_id = ?2 AND is_active = true"
    )
    .bind(user_id.to_string())
    .bind(org_id.to_string())
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        let role = match row.get::<String, _>("role").as_str() {
            "owner" => OrganizationRole::Owner,
            "admin" => OrganizationRole::Admin,
            _ => OrganizationRole::Member,
        };
        Ok(Some(role))
    } else {
        Ok(None)
    }
}

pub async fn is_user_organization_member(
    pool: &SqlitePool, 
    email: &str, 
    org_id: Uuid
) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM organization_members om
        JOIN users u ON om.user_id = u.id
        WHERE u.email = ?1 AND om.organization_id = ?2 AND om.is_active = true
        "#
    )
    .bind(email)
    .bind(org_id.to_string())
    .fetch_one(pool)
    .await?;
    
    Ok(count > 0)
}

pub async fn create_organization_invite(
    pool: &SqlitePool,
    org_id: Uuid,
    email: &str,
    role: OrganizationRole,
    invited_by: Uuid,
) -> Result<OrganizationInvite> {
    let id = Uuid::new_v4();
    let token = Uuid::new_v4().to_string();
    let now = Utc::now();
    let expires_at = now + chrono::Duration::days(7);

    let role_str = match role {
        OrganizationRole::Owner => "owner",
        OrganizationRole::Admin => "admin",
        OrganizationRole::Member => "member",
        OrganizationRole::Viewer => "viewer",
    };

    sqlx::query(
        r#"
        INSERT INTO organization_invites (id, organization_id, email, role, invited_by, token, expires_at, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#
    )
    .bind(id.to_string())
    .bind(org_id.to_string())
    .bind(email)
    .bind(role_str)
    .bind(invited_by.to_string())
    .bind(&token)
    .bind(expires_at.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(OrganizationInvite {
        id,
        organization_id: org_id,
        email: email.to_string(),
        role,
        invited_by,
        token,
        expires_at,
        created_at: now,
        accepted_at: None,
    })
}

pub async fn get_organization_invite_by_token(
    pool: &SqlitePool, 
    token: &str
) -> Result<Option<OrganizationInvite>> {
    let row = sqlx::query(
        "SELECT id, organization_id, email, role, invited_by, token, expires_at, created_at, accepted_at FROM organization_invites WHERE token = ?1 AND expires_at > ?2"
    )
    .bind(token)
    .bind(Utc::now().to_rfc3339())
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        let role = match row.get::<String, _>("role").as_str() {
            "owner" => OrganizationRole::Owner,
            "admin" => OrganizationRole::Admin,
            _ => OrganizationRole::Member,
        };

        Ok(Some(OrganizationInvite {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            organization_id: Uuid::parse_str(&row.get::<String, _>("organization_id"))?,
            email: row.get("email"),
            role,
            invited_by: Uuid::parse_str(&row.get::<String, _>("invited_by"))?,
            token: row.get("token"),
            expires_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("expires_at")).unwrap().with_timezone(&chrono::Utc),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at")).unwrap().with_timezone(&chrono::Utc),
            accepted_at: row.get::<Option<String>, _>("accepted_at").map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
        }))
    } else {
        Ok(None)
    }
}

pub async fn accept_organization_invite(
    pool: &SqlitePool, 
    invite_id: Uuid, 
    user_id: Uuid
) -> Result<OrganizationMember> {
    let invite = sqlx::query(
        "SELECT organization_id, role, invited_by FROM organization_invites WHERE id = ?1"
    )
    .bind(invite_id.to_string())
    .fetch_one(pool)
    .await?;

    let org_id = Uuid::parse_str(&invite.get::<String, _>("organization_id"))?;
    let role = match invite.get::<String, _>("role").as_str() {
        "owner" => OrganizationRole::Owner,
        "admin" => OrganizationRole::Admin,
        _ => OrganizationRole::Member,
    };
    let invited_by = Uuid::parse_str(&invite.get::<String, _>("invited_by"))?;

    let member_id = Uuid::new_v4();
    let now = Utc::now();

    // Create organization member
    sqlx::query(
        r#"
        INSERT INTO organization_members (id, organization_id, user_id, role, invited_by, invited_at, joined_at, is_active)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#
    )
    .bind(member_id.to_string())
    .bind(org_id.to_string())
    .bind(user_id.to_string())
    .bind(match role {
        OrganizationRole::Owner => "owner",
        OrganizationRole::Admin => "admin",
        OrganizationRole::Member => "member",
        OrganizationRole::Viewer => "viewer",
    })
    .bind(invited_by.to_string())
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .bind(true)
    .execute(pool)
    .await?;

    // Mark invite as accepted
    sqlx::query(
        "UPDATE organization_invites SET accepted_at = ?1 WHERE id = ?2"
    )
    .bind(now.to_rfc3339())
    .bind(invite_id.to_string())
    .execute(pool)
    .await?;

    Ok(OrganizationMember {
        id: member_id,
        organization_id: org_id,
        user_id,
        role,
        invited_by: Some(invited_by),
        invited_at: now,
        joined_at: Some(now),
        is_active: true,
    })
}

pub async fn get_organization_member(
    pool: &SqlitePool, 
    member_id: Uuid
) -> Result<Option<OrganizationMember>> {
    let row = sqlx::query(
        "SELECT id, organization_id, user_id, role, invited_by, invited_at, joined_at, is_active FROM organization_members WHERE id = ?1"
    )
    .bind(member_id.to_string())
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        let role = match row.get::<String, _>("role").as_str() {
            "owner" => OrganizationRole::Owner,
            "admin" => OrganizationRole::Admin,
            _ => OrganizationRole::Member,
        };

        Ok(Some(OrganizationMember {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            organization_id: Uuid::parse_str(&row.get::<String, _>("organization_id"))?,
            user_id: Uuid::parse_str(&row.get::<String, _>("user_id"))?,
            role,
            invited_by: row.get::<Option<String>, _>("invited_by").map(|s| Uuid::parse_str(&s)).transpose()?,
            invited_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("invited_at")).unwrap().with_timezone(&chrono::Utc),
            joined_at: row.get::<Option<String>, _>("joined_at").map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            is_active: row.get("is_active"),
        }))
    } else {
        Ok(None)
    }
}

pub async fn remove_organization_member(pool: &SqlitePool, member_id: Uuid) -> Result<()> {
    sqlx::query("UPDATE organization_members SET is_active = false WHERE id = ?1")
        .bind(member_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_user_organization_membership(
    pool: &SqlitePool, 
    user_id: Uuid, 
    org_id: Uuid
) -> Result<Option<OrganizationMember>> {
    let row = sqlx::query(
        "SELECT id, organization_id, user_id, role, invited_by, invited_at, joined_at, is_active FROM organization_members WHERE user_id = ?1 AND organization_id = ?2 AND is_active = true"
    )
    .bind(user_id.to_string())
    .bind(org_id.to_string())
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        let role = match row.get::<String, _>("role").as_str() {
            "owner" => OrganizationRole::Owner,
            "admin" => OrganizationRole::Admin,
            _ => OrganizationRole::Member,
        };

        Ok(Some(OrganizationMember {
            id: Uuid::parse_str(&row.get::<String, _>("id"))?,
            organization_id: Uuid::parse_str(&row.get::<String, _>("organization_id"))?,
            user_id: Uuid::parse_str(&row.get::<String, _>("user_id"))?,
            role,
            invited_by: row.get::<Option<String>, _>("invited_by").map(|s| Uuid::parse_str(&s)).transpose()?,
            invited_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("invited_at")).unwrap().with_timezone(&chrono::Utc),
            joined_at: row.get::<Option<String>, _>("joined_at").map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            is_active: row.get("is_active"),
        }))
    } else {
        Ok(None)
    }
}
