use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
#[cfg(feature = "ssr")]
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub website: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrganizationMember {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub role: OrganizationRole,
    pub invited_by: Option<Uuid>,
    pub invited_at: DateTime<Utc>,
    pub joined_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "organization_role")]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum OrganizationRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrganizationInvite {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub email: String,
    pub role: OrganizationRole,
    pub invited_by: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
}

// Request/Response DTOs
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ssr", derive(Validate))]
pub struct CreateOrganizationRequest {
    #[cfg_attr(feature = "ssr", validate(length(min = 2, max = 50)))]
    pub name: String,
    
    #[cfg_attr(feature = "ssr", validate(length(min = 1, max = 100)))]
    pub display_name: String,
    
    #[cfg_attr(feature = "ssr", validate(length(max = 500)))]
    pub description: Option<String>,
    
    #[cfg_attr(feature = "ssr", validate(url))]
    pub website: Option<String>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ssr", derive(Validate))]
pub struct UpdateOrganizationRequest {
    #[cfg_attr(feature = "ssr", validate(length(min = 1, max = 100)))]
    pub display_name: Option<String>,
    
    #[cfg_attr(feature = "ssr", validate(length(max = 500)))]
    pub description: Option<String>,
    
    #[cfg_attr(feature = "ssr", validate(url))]
    pub website: Option<String>,
    
    #[cfg_attr(feature = "ssr", validate(url))]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ssr", derive(Validate))]
pub struct InviteUserRequest {
    #[cfg_attr(feature = "ssr", validate(email))]
    pub email: String,
    pub role: OrganizationRole,
}

#[derive(Debug, Serialize)]
pub struct OrganizationResponse {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub website: Option<String>,
    pub owner: BasicUserResponse,
    pub member_count: i64,
    pub crate_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct OrganizationMemberResponse {
    pub id: Uuid,
    pub user: BasicUserResponse,
    pub role: OrganizationRole,
    pub joined_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct BasicUserResponse {
    pub id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OrganizationInviteResponse {
    pub id: Uuid,
    pub organization: BasicOrganizationResponse,
    pub email: String,
    pub role: OrganizationRole,
    pub invited_by: BasicUserResponse,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BasicOrganizationResponse {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

impl From<Organization> for OrganizationResponse {
    fn from(org: Organization) -> Self {
        Self {
            id: org.id,
            name: org.name,
            display_name: org.display_name,
            description: org.description,
            avatar_url: org.avatar_url,
            website: org.website,
            owner: BasicUserResponse {
                id: org.owner_id,
                username: String::new(), // Will be filled by the handler
                avatar_url: None,
            },
            member_count: 0, // Will be filled by the handler
            crate_count: 0,  // Will be filled by the handler
            created_at: org.created_at,
            updated_at: org.updated_at,
        }
    }
}

impl OrganizationRole {
    pub fn can_invite(&self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }

    pub fn can_manage_members(&self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }

    pub fn can_publish_crates(&self) -> bool {
        matches!(self, Self::Owner | Self::Admin | Self::Member)
    }

    pub fn can_delete_organization(&self) -> bool {
        matches!(self, Self::Owner)
    }

    pub fn can_transfer_ownership(&self) -> bool {
        matches!(self, Self::Owner)
    }
}
