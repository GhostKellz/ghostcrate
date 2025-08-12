# GitHub OIDC Setup Guide for GhostCrate

This guide explains how to configure GitHub as an OIDC provider for GhostCrate, enabling GitHub-based authentication and organization access control.

## 📋 Prerequisites

- GitHub account with organization admin access (for org restrictions)
- GhostCrate v0.2.0+ deployed and accessible
- Domain name for your GhostCrate instance (e.g., `crates.cktech.org`)

## 🔧 Step 1: Create GitHub OAuth App

### 1.1 Navigate to GitHub Settings
1. Go to [GitHub.com](https://github.com)
2. Click your profile picture → **Settings**
3. Scroll down to **Developer settings**
4. Click **OAuth Apps**
5. Click **New OAuth App**

### 1.2 Configure OAuth Application
Fill in the application details:

**Application name**: `GhostCrate Registry`

**Homepage URL**: `https://crates.cktech.org`

**Application description**: `Self-hosted Rust crate registry with GitHub authentication`

**Authorization callback URL**: `https://crates.cktech.org/auth/oidc/github/callback`

⚠️ **Important**: Replace `crates.cktech.org` with your actual domain

### 1.3 Save and Note Credentials
After creating the app:
1. Note the **Client ID** - You'll need this as `GHOSTCRATE_OIDC_GITHUB_CLIENT_ID`
2. Click **Generate a new client secret**
3. Copy the **Client secret** - You'll need this as `GHOSTCRATE_OIDC_GITHUB_CLIENT_SECRET`

## 🏢 Step 2: Configure Organization Access (Optional)

### 2.1 For Organization-Wide Deployment
If you want to restrict access to specific GitHub organizations:

1. Go to your GitHub organization
2. Navigate to **Settings** → **Third-party access**
3. Find your OAuth app and approve it
4. Configure access policies as needed

### 2.2 Team-Based Access
Create teams for different access levels:
- `ghostcrate-users` - Regular users
- `ghostcrate-admins` - Administrator access
- `ghostcrate-publishers` - Can publish crates

## 📝 Step 3: Configure GhostCrate Environment

### 3.1 Docker Compose Configuration
Add to your `docker-compose.yml`:

```yaml
environment:
  # Enable GitHub OIDC
  - GHOSTCRATE_OIDC_GITHUB_CLIENT_ID=your-github-client-id
  - GHOSTCRATE_OIDC_GITHUB_CLIENT_SECRET=your-github-client-secret
  - GHOSTCRATE_OIDC_GITHUB_REDIRECT_URI=https://crates.cktech.org/auth/oidc/github/callback
  - GHOSTCRATE_OIDC_GITHUB_AUTO_REGISTER=true
  - GHOSTCRATE_OIDC_GITHUB_SCOPES=user:email,read:org
  
  # Optional: Restrict to specific organizations
  # - GHOSTCRATE_OIDC_GITHUB_ALLOWED_ORGS=your-org,another-org
  
  # Optional: Admin teams (format: org/team)
  # - GHOSTCRATE_OIDC_GITHUB_ADMIN_TEAMS=your-org/admins,your-org/maintainers
```

### 3.2 Environment File Configuration
Or create `.env.production`:

```bash
# GitHub OIDC Configuration
GHOSTCRATE_OIDC_GITHUB_CLIENT_ID=Iv1.1234567890abcdef
GHOSTCRATE_OIDC_GITHUB_CLIENT_SECRET=1234567890abcdef1234567890abcdef12345678
GHOSTCRATE_OIDC_GITHUB_REDIRECT_URI=https://crates.cktech.org/auth/oidc/github/callback
GHOSTCRATE_OIDC_GITHUB_AUTO_REGISTER=true
GHOSTCRATE_OIDC_GITHUB_SCOPES=user:email,read:org

# Optional: Organization restrictions
GHOSTCRATE_OIDC_GITHUB_ALLOWED_ORGS=your-org,partner-org
GHOSTCRATE_OIDC_GITHUB_ADMIN_TEAMS=your-org/admins
```

## 🚀 Step 4: Deploy and Test

### 4.1 Deploy GhostCrate
```bash
docker-compose down
docker-compose up -d
```

### 4.2 Test GitHub Login
1. Navigate to your GhostCrate instance
2. Look for "Sign in with GitHub" button
3. Test the authentication flow

### 4.3 Direct Test URL
```
https://crates.cktech.org/auth/oidc/login?provider=github
```

## 🔍 Available Scopes

Configure scopes based on your needs:

### Basic Scopes
- `user:email` - Access user email (required)
- `read:user` - Read user profile information

### Organization Scopes
- `read:org` - Read organization membership
- `read:team` - Read team membership (for team-based access)

### Repository Scopes (for future features)
- `repo` - Full repository access
- `public_repo` - Public repository access only

## 🔐 Security Considerations

### 1. Scope Minimization
Only request the minimum scopes needed:
```bash
# Minimal setup
GHOSTCRATE_OIDC_GITHUB_SCOPES=user:email

# With organization support  
GHOSTCRATE_OIDC_GITHUB_SCOPES=user:email,read:org
```

### 2. Organization Restrictions
Limit access to specific organizations:
```bash
GHOSTCRATE_OIDC_GITHUB_ALLOWED_ORGS=your-company,trusted-partner
```

### 3. Team-Based Authorization
Use GitHub teams for role-based access:
```bash
GHOSTCRATE_OIDC_GITHUB_ADMIN_TEAMS=your-org/crate-admins,your-org/devops
GHOSTCRATE_OIDC_GITHUB_PUBLISHER_TEAMS=your-org/developers,your-org/maintainers
```

## 🔄 Advanced Configuration

### Enterprise GitHub
For GitHub Enterprise Server:

```bash
# GitHub Enterprise configuration
GHOSTCRATE_OIDC_GITHUB_BASE_URL=https://github.your-company.com
GHOSTCRATE_OIDC_GITHUB_API_URL=https://github.your-company.com/api/v3
```

### Webhook Integration
Set up webhooks for real-time updates:

1. Go to your organization **Settings** → **Webhooks**
2. Add webhook URL: `https://crates.cktech.org/webhooks/github`
3. Select events: `Member`, `Team`, `Organization`

## 🔍 Troubleshooting

### Common Issues

#### 1. "Invalid client_id"
**Solution**: Verify client ID matches GitHub OAuth app

#### 2. "Redirect URI mismatch"
**Solution**: Ensure callback URL in GitHub app exactly matches environment variable

#### 3. "Access denied" for organization members
**Solution**: 
- Check organization OAuth app approval
- Verify user is member of allowed organizations
- Check team membership for team-based access

#### 4. Missing email address
**Solution**: 
- Ensure `user:email` scope is included
- User must have public email or grant email permission

### Debug Commands

Check logs:
```bash
docker-compose logs -f ghostcrate | grep -i github
```

Test API access:
```bash
curl -H "Authorization: token YOUR_TOKEN" https://api.github.com/user
```

## 📋 Configuration Checklist

- [ ] GitHub OAuth app created
- [ ] Client ID and secret configured in GhostCrate
- [ ] Redirect URI matches exactly
- [ ] Appropriate scopes configured
- [ ] Organization access approved (if using org restrictions)
- [ ] Team-based access configured (if needed)
- [ ] GhostCrate restarted with new configuration
- [ ] GitHub login tested successfully
- [ ] Organization/team restrictions tested

---

**🎉 Success!** GitHub OIDC is now configured for GhostCrate. Users can authenticate with their GitHub accounts and access is controlled through GitHub organizations and teams.
