use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// OIDC Provider Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcProvider {
    pub id: Uuid,
    pub name: String,
    pub provider_type: OidcProviderType,
    pub client_id: String,
    pub client_secret: String,
    pub discovery_url: String,
    pub scopes: Vec<String>,
    pub enabled: bool,
    pub auto_register: bool,
    pub default_role: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OidcProviderType {
    EntraId,     // Microsoft Entra ID (Azure AD)
    GitHub,      // GitHub OAuth
    Google,      // Google OAuth
    Okta,        // Okta
    Auth0,       // Auth0
    Generic,     // Generic OIDC provider
}

/// OIDC User Claims from ID Token
#[derive(Debug, Serialize, Deserialize)]
pub struct OidcClaims {
    pub sub: String,                    // Subject (user ID)
    pub iss: String,                    // Issuer
    pub aud: String,                    // Audience
    pub exp: i64,                       // Expiration
    pub iat: i64,                       // Issued at
    pub name: Option<String>,           // Full name
    pub given_name: Option<String>,     // First name
    pub family_name: Option<String>,    // Last name
    pub preferred_username: Option<String>, // Username
    pub email: Option<String>,          // Email
    pub email_verified: Option<bool>,   // Email verified
    pub picture: Option<String>,        // Profile picture
    pub groups: Option<Vec<String>>,    // Groups (for Entra ID)
    pub roles: Option<Vec<String>>,     // Roles (for Entra ID)
    pub upn: Option<String>,            // User Principal Name (Entra ID)
    pub tenant_id: Option<String>,      // Tenant ID (Entra ID)
}

/// OIDC User Link - links local users to external OIDC providers
#[derive(Debug, Serialize, Deserialize)]
pub struct OidcUserLink {
    pub id: Uuid,
    pub user_id: Uuid,                  // Local user ID
    pub provider_id: Uuid,              // OIDC provider ID
    pub external_id: String,            // Subject from OIDC provider
    pub email: String,                  // Email from provider
    pub name: Option<String>,           // Display name
    pub avatar_url: Option<String>,     // Profile picture
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// OIDC Login Request
#[derive(Debug, Deserialize)]
pub struct OidcLoginRequest {
    pub provider: String,               // Provider name
    pub return_url: Option<String>,     // Where to redirect after auth
}

/// OIDC Callback Request
#[derive(Debug, Deserialize)]
pub struct OidcCallbackRequest {
    pub code: String,                   // Authorization code
    pub state: String,                  // State parameter
    pub session_state: Option<String>,  // Session state (Entra ID)
}

/// OIDC Configuration for different providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    pub entra_id: Option<EntraIdConfig>,
    pub github: Option<GitHubOidcConfig>,
    pub google: Option<GoogleConfig>,
    pub generic_providers: Vec<GenericOidcConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntraIdConfig {
    pub client_id: String,
    pub client_secret: String,
    pub tenant_id: String,              // Azure tenant ID
    pub redirect_uri: String,
    pub scopes: Vec<String>,            // Default: ["openid", "profile", "email", "User.Read"]
    pub auto_register: bool,
    pub required_groups: Option<Vec<String>>, // Required AD groups
    pub admin_groups: Option<Vec<String>>,    // Groups that get admin access
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubOidcConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,            // Default: ["user:email", "read:org"]
    pub auto_register: bool,
    pub allowed_organizations: Option<Vec<String>>, // Restrict to specific orgs
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,            // Default: ["openid", "profile", "email"]
    pub auto_register: bool,
    pub allowed_domains: Option<Vec<String>>, // Restrict to specific domains
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericOidcConfig {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub discovery_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub auto_register: bool,
    pub claim_mappings: OidcClaimMappings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcClaimMappings {
    pub username: String,               // Claim name for username (default: "preferred_username")
    pub email: String,                  // Claim name for email (default: "email")
    pub name: String,                   // Claim name for display name (default: "name")
    pub groups: Option<String>,         // Claim name for groups
    pub roles: Option<String>,          // Claim name for roles
}

impl Default for OidcClaimMappings {
    fn default() -> Self {
        Self {
            username: "preferred_username".to_string(),
            email: "email".to_string(),
            name: "name".to_string(),
            groups: Some("groups".to_string()),
            roles: Some("roles".to_string()),
        }
    }
}

impl Default for EntraIdConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            tenant_id: String::new(),
            redirect_uri: String::new(),
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
                "User.Read".to_string(),
            ],
            auto_register: true,
            required_groups: None,
            admin_groups: None,
        }
    }
}

impl Default for GitHubOidcConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: String::new(),
            scopes: vec![
                "user:email".to_string(),
                "read:org".to_string(),
            ],
            auto_register: true,
            allowed_organizations: None,
        }
    }
}

impl Default for GoogleConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: String::new(),
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
            ],
            auto_register: true,
            allowed_domains: None,
        }
    }
}
