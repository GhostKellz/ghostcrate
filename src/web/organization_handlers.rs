use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;
use tracing::{info, error, warn};

use crate::models::{
    User, Organization, OrganizationMember, OrganizationRole, OrganizationInvite,
    CreateOrganizationRequest, UpdateOrganizationRequest, InviteUserRequest,
    OrganizationResponse, OrganizationMemberResponse, OrganizationInviteResponse,
    BasicUserResponse, BasicOrganizationResponse
};
use crate::{AppState, db};

#[derive(Debug, Deserialize)]
pub struct OrganizationQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct AcceptInviteRequest {
    pub token: String,
}

#[cfg(feature = "ssr")]
pub async fn create_organization_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
    Json(request): Json<CreateOrganizationRequest>,
) -> Result<Json<OrganizationResponse>, StatusCode> {
    if !app_state.config.registry.organizations_enabled {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }

    // Validate request (only in SSR mode)
    #[cfg(feature = "ssr")]
    request.validate().map_err(|_| StatusCode::BAD_REQUEST)?;

    // Check if organization name is available
    if db::organization_exists(&app_state.pool, &request.name).await.unwrap_or(true) {
        return Err(StatusCode::CONFLICT);
    }

    let organization = db::create_organization(
        &app_state.pool,
        &request,
        user.id,
    ).await.map_err(|e| {
        error!("Failed to create organization: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!("User {} created organization: {}", user.username, organization.name);

    let mut response: OrganizationResponse = organization.into();
    response.owner = BasicUserResponse {
        id: user.id,
        username: user.username,
        avatar_url: None, // TODO: Add avatar support
    };
    response.member_count = 1; // Owner is automatically a member

    Ok(Json(response))
}

#[cfg(feature = "ssr")]
pub async fn get_organization_handler(
    State(app_state): State<AppState>,
    Path(org_name): Path<String>,
) -> Result<Json<OrganizationResponse>, StatusCode> {
    let organization = db::get_organization_by_name(&app_state.pool, &org_name)
        .await
        .map_err(|e| {
            error!("Database error getting organization: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let owner = db::get_user_by_id(&app_state.pool, organization.owner_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let member_count = db::get_organization_member_count(&app_state.pool, organization.id)
        .await
        .unwrap_or(0);

    let crate_count = db::get_organization_crate_count(&app_state.pool, organization.id)
        .await
        .unwrap_or(0);

    let mut response: OrganizationResponse = organization.into();
    response.owner = BasicUserResponse {
        id: owner.id,
        username: owner.username,
        avatar_url: None,
    };
    response.member_count = member_count;
    response.crate_count = crate_count;

    Ok(Json(response))
}

#[cfg(feature = "ssr")]
pub async fn update_organization_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
    Path(org_name): Path<String>,
    Json(request): Json<UpdateOrganizationRequest>,
) -> Result<Json<OrganizationResponse>, StatusCode> {
    request.validate().map_err(|_| StatusCode::BAD_REQUEST)?;

    let organization = db::get_organization_by_name(&app_state.pool, &org_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if user has permission to update organization
    if !db::user_can_manage_organization(&app_state.pool, user.id, organization.id).await.unwrap_or(false) {
        return Err(StatusCode::FORBIDDEN);
    }

    let updated_organization = db::update_organization(
        &app_state.pool,
        organization.id,
        &request,
    ).await.map_err(|e| {
        error!("Failed to update organization: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!("User {} updated organization: {}", user.username, organization.name);

    let owner = db::get_user_by_id(&app_state.pool, updated_organization.owner_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let member_count = db::get_organization_member_count(&app_state.pool, updated_organization.id)
        .await
        .unwrap_or(0);

    let crate_count = db::get_organization_crate_count(&app_state.pool, updated_organization.id)
        .await
        .unwrap_or(0);

    let mut response: OrganizationResponse = updated_organization.into();
    response.owner = BasicUserResponse {
        id: owner.id,
        username: owner.username,
        avatar_url: None,
    };
    response.member_count = member_count;
    response.crate_count = crate_count;

    Ok(Json(response))
}

#[cfg(feature = "ssr")]
pub async fn delete_organization_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
    Path(org_name): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let organization = db::get_organization_by_name(&app_state.pool, &org_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Only owner can delete organization
    if organization.owner_id != user.id {
        return Err(StatusCode::FORBIDDEN);
    }

    db::delete_organization(&app_state.pool, organization.id)
        .await
        .map_err(|e| {
            error!("Failed to delete organization: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("User {} deleted organization: {}", user.username, organization.name);
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(feature = "ssr")]
pub async fn get_organization_members_handler(
    State(app_state): State<AppState>,
    Path(org_name): Path<String>,
    Query(params): Query<OrganizationQuery>,
) -> Result<Json<Vec<OrganizationMemberResponse>>, StatusCode> {
    let organization = db::get_organization_by_name(&app_state.pool, &org_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let per_page = params.per_page.unwrap_or(50).min(100) as i64;
    let page = params.page.unwrap_or(1) as i64;
    let offset = (page - 1) * per_page;

    let members = db::get_organization_members(&app_state.pool, organization.id, per_page, offset)
        .await
        .map_err(|e| {
            error!("Failed to get organization members: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut member_responses = Vec::new();
    for (member, user) in members {
        member_responses.push(OrganizationMemberResponse {
            id: member.id,
            user: BasicUserResponse {
                id: user.id,
                username: user.username,
                avatar_url: None,
            },
            role: member.role,
            joined_at: member.joined_at,
            is_active: member.is_active,
        });
    }

    Ok(Json(member_responses))
}

#[cfg(feature = "ssr")]
pub async fn invite_user_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
    Path(org_name): Path<String>,
    Json(request): Json<InviteUserRequest>,
) -> Result<Json<OrganizationInviteResponse>, StatusCode> {
    request.validate().map_err(|_| StatusCode::BAD_REQUEST)?;

    let organization = db::get_organization_by_name(&app_state.pool, &org_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if user has permission to invite
    let user_role = db::get_user_organization_role(&app_state.pool, user.id, organization.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::FORBIDDEN)?;

    if !user_role.can_invite() {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if user is already a member or has pending invite
    if db::is_user_organization_member(&app_state.pool, &request.email, organization.id).await.unwrap_or(false) {
        return Err(StatusCode::CONFLICT);
    }

    let invite = db::create_organization_invite(
        &app_state.pool,
        organization.id,
        &request.email,
        request.role,
        user.id,
    ).await.map_err(|e| {
        error!("Failed to create organization invite: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!("User {} invited {} to organization {}", user.username, request.email, organization.name);

    // TODO: Send email invitation

    let response = OrganizationInviteResponse {
        id: invite.id,
        organization: BasicOrganizationResponse {
            id: organization.id,
            name: organization.name,
            display_name: organization.display_name,
            avatar_url: organization.avatar_url,
        },
        email: invite.email,
        role: invite.role,
        invited_by: BasicUserResponse {
            id: user.id,
            username: user.username,
            avatar_url: None,
        },
        expires_at: invite.expires_at,
        created_at: invite.created_at,
    };

    Ok(Json(response))
}

#[cfg(feature = "ssr")]
pub async fn accept_invite_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
    Json(request): Json<AcceptInviteRequest>,
) -> Result<Json<OrganizationMemberResponse>, StatusCode> {
    let invite = db::get_organization_invite_by_token(&app_state.pool, &request.token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if invite has expired
    if invite.expires_at < chrono::Utc::now() {
        return Err(StatusCode::GONE);
    }

    // Check if the user's email matches the invite
    if user.email != invite.email {
        return Err(StatusCode::FORBIDDEN);
    }

    let member = db::accept_organization_invite(&app_state.pool, invite.id, user.id)
        .await
        .map_err(|e| {
            error!("Failed to accept organization invite: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("User {} accepted invitation to organization", user.username);

    let response = OrganizationMemberResponse {
        id: member.id,
        user: BasicUserResponse {
            id: user.id,
            username: user.username,
            avatar_url: None,
        },
        role: member.role,
        joined_at: member.joined_at,
        is_active: member.is_active,
    };

    Ok(Json(response))
}

#[cfg(feature = "ssr")]
pub async fn remove_member_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
    Path((org_name, member_id)): Path<(String, Uuid)>,
) -> Result<StatusCode, StatusCode> {
    let organization = db::get_organization_by_name(&app_state.pool, &org_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if user has permission to remove members
    let user_role = db::get_user_organization_role(&app_state.pool, user.id, organization.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::FORBIDDEN)?;

    if !user_role.can_manage_members() {
        return Err(StatusCode::FORBIDDEN);
    }

    // Don't allow removing the organization owner
    let member = db::get_organization_member(&app_state.pool, member_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if member.user_id == organization.owner_id {
        return Err(StatusCode::BAD_REQUEST);
    }

    db::remove_organization_member(&app_state.pool, member_id)
        .await
        .map_err(|e| {
            error!("Failed to remove organization member: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("User {} removed member from organization {}", user.username, organization.name);
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(feature = "ssr")]
pub async fn leave_organization_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
    Path(org_name): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let organization = db::get_organization_by_name(&app_state.pool, &org_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Owner cannot leave their own organization
    if organization.owner_id == user.id {
        return Err(StatusCode::BAD_REQUEST);
    }

    let member = db::get_user_organization_membership(&app_state.pool, user.id, organization.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    db::remove_organization_member(&app_state.pool, member.id)
        .await
        .map_err(|e| {
            error!("Failed to leave organization: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("User {} left organization {}", user.username, organization.name);
    Ok(StatusCode::NO_CONTENT)
}
