# OIDC Azure Entra ID Setup Guide for GhostCrate

This guide walks you through setting up Microsoft Entra ID (formerly Azure Active Directory) as an OIDC provider for GhostCrate, enabling single sign-on (SSO) for your organization.

## 📋 Prerequisites

- Azure subscription with Entra ID tenant
- Admin access to Azure Entra ID
- GhostCrate v0.2.0+ deployed and accessible
- Domain name for your GhostCrate instance (e.g., `crates.cktech.org`)

## 🔧 Step 1: Create App Registration in Azure

### 1.1 Navigate to Azure Portal
1. Go to [Azure Portal](https://portal.azure.com)
2. Navigate to **Azure Active Directory** (now called **Microsoft Entra ID**)
3. Click **App registrations** in the left sidebar
4. Click **New registration**

### 1.2 Configure App Registration
Fill in the following details:

**Name**: `GhostCrate Registry`

**Supported account types**: Choose based on your needs:
- `Accounts in this organizational directory only` - Single tenant (recommended for internal use)
- `Accounts in any organizational directory` - Multi-tenant
- `Accounts in any organizational directory and personal Microsoft accounts` - Public

**Redirect URI**: 
- Platform: `Web`
- URI: `https://crates.cktech.org/auth/oidc/entra/callback`
  
  ⚠️ **Important**: Replace `crates.cktech.org` with your actual domain

### 1.3 Save and Note Details
After creating the registration, note down:
- **Application (client) ID** - You'll need this as `GHOSTCRATE_OIDC_ENTRAID_CLIENT_ID`
- **Directory (tenant) ID** - You'll need this as `GHOSTCRATE_OIDC_ENTRAID_TENANT_ID`

## 🔐 Step 2: Create Client Secret

1. In your app registration, go to **Certificates & secrets**
2. Click **New client secret**
3. Add a description: `GhostCrate OIDC Secret`
4. Choose expiration (recommend 24 months for production)
5. Click **Add**
6. **Copy the secret value immediately** - You'll need this as `GHOSTCRATE_OIDC_ENTRAID_CLIENT_SECRET`

⚠️ **Warning**: The secret value is only shown once. Store it securely!

## 🔧 Step 3: Configure API Permissions

### 3.1 Add Required Permissions
1. Go to **API permissions**
2. Click **Add a permission**
3. Select **Microsoft Graph**
4. Choose **Delegated permissions**
5. Add these permissions:
   - `openid` (Sign users in)
   - `profile` (View users' basic profile)
   - `email` (View users' email address)
   - `User.Read` (Sign in and read user profile)

### 3.2 Optional: Add Group Claims (for role-based access)
If you want to use Azure AD groups for authorization:
1. Add permission: `GroupMember.Read.All`
2. Go to **Token configuration**
3. Click **Add groups claim**
4. Select **Security groups**
5. For ID tokens, choose **Group ID**

### 3.3 Grant Admin Consent
1. Click **Grant admin consent for [Your Organization]**
2. Confirm by clicking **Yes**

## 🏢 Step 4: Configure Optional Group-Based Access

### 4.1 Create Security Groups (Optional)
Create groups for different access levels:

1. Go to **Azure Active Directory** > **Groups**
2. Click **New group**
3. Create groups like:
   - `GhostCrate-Users` - Regular users
   - `GhostCrate-Admins` - Administrator access
   - `GhostCrate-Publishers` - Can publish crates

### 4.2 Add Users to Groups
1. Select each group
2. Click **Members** > **Add members**
3. Add appropriate users

## 📝 Step 5: Configure GhostCrate Environment

Add these environment variables to your GhostCrate deployment:

### 5.1 Docker Compose Configuration
Add to your `docker-compose.yml`:

```yaml
environment:
  # Enable Entra ID OIDC
  - GHOSTCRATE_OIDC_ENTRAID_CLIENT_ID=your-application-client-id
  - GHOSTCRATE_OIDC_ENTRAID_CLIENT_SECRET=your-client-secret
  - GHOSTCRATE_OIDC_ENTRAID_TENANT_ID=your-tenant-id
  - GHOSTCRATE_OIDC_ENTRAID_REDIRECT_URI=https://crates.cktech.org/auth/oidc/entra/callback
  - GHOSTCRATE_OIDC_ENTRAID_AUTO_REGISTER=true
  - GHOSTCRATE_OIDC_ENTRAID_SCOPES=openid,profile,email,User.Read
  
  # Optional: Group-based access control
  # - GHOSTCRATE_OIDC_ENTRAID_REQUIRED_GROUPS=GhostCrate-Users
  # - GHOSTCRATE_OIDC_ENTRAID_ADMIN_GROUPS=GhostCrate-Admins
```

### 5.2 Environment File Configuration
Or create `.env.production`:

```bash
# Entra ID OIDC Configuration
GHOSTCRATE_OIDC_ENTRAID_CLIENT_ID=12345678-1234-1234-1234-123456789012
GHOSTCRATE_OIDC_ENTRAID_CLIENT_SECRET=your-secret-value-here
GHOSTCRATE_OIDC_ENTRAID_TENANT_ID=87654321-4321-4321-4321-210987654321
GHOSTCRATE_OIDC_ENTRAID_REDIRECT_URI=https://crates.cktech.org/auth/oidc/entra/callback
GHOSTCRATE_OIDC_ENTRAID_AUTO_REGISTER=true
GHOSTCRATE_OIDC_ENTRAID_SCOPES=openid,profile,email,User.Read

# Optional: Multi-tenant configuration
# GHOSTCRATE_OIDC_ENTRAID_AUTHORITY=https://login.microsoftonline.com/common
```

## 🔧 Step 6: Update Nginx Configuration

Add OIDC callback routes to your nginx configuration:

```nginx
# OIDC callback endpoints
location /auth/oidc/ {
    proxy_pass http://10.0.0.38:8080/auth/oidc/;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    
    # OIDC flows can take time
    proxy_connect_timeout 60s;
    proxy_send_timeout 60s;
    proxy_read_timeout 60s;
}
```

## 🚀 Step 7: Deploy and Test

### 7.1 Deploy GhostCrate
1. Update your docker-compose.yml with the new environment variables
2. Restart GhostCrate:
   ```bash
   docker-compose down
   docker-compose up -d
   ```

### 7.2 Test OIDC Login
1. Navigate to your GhostCrate instance
2. Look for "Sign in with Microsoft" button
3. Click and verify redirect to Microsoft login
4. Enter your Azure AD credentials
5. Verify successful login and user creation

### 7.3 Test Authorization URL
Direct test URL:
```
https://crates.cktech.org/auth/oidc/login?provider=entra
```

## 🔍 Troubleshooting

### Common Issues

#### 1. "AADSTS50011: Reply URL Mismatch"
**Solution**: Ensure redirect URI in Azure exactly matches your GhostCrate callback URL
- Azure: `https://crates.cktech.org/auth/oidc/entra/callback`
- Environment: `GHOSTCRATE_OIDC_ENTRAID_REDIRECT_URI=https://crates.cktech.org/auth/oidc/entra/callback`

#### 2. "AADSTS7000218: Invalid client assertion"
**Solution**: Check client secret hasn't expired and is correctly set

#### 3. "Invalid client_id"
**Solution**: Verify `GHOSTCRATE_OIDC_ENTRAID_CLIENT_ID` matches Application ID from Azure

#### 4. Users Can't Access After Login
**Solution**: 
- Check `GHOSTCRATE_OIDC_ENTRAID_AUTO_REGISTER=true`
- Verify user email domains are allowed
- Check group membership if using group-based access

### Debug Mode
Enable debug logging:
```bash
RUST_LOG=debug
```

Check logs for OIDC flow details:
```bash
docker-compose logs -f ghostcrate | grep -i oidc
```

## 🔐 Security Best Practices

### 1. Client Secret Management
- Store secrets in environment variables, never in code
- Rotate client secrets regularly (recommend every 6-12 months)
- Use Azure Key Vault in production environments

### 2. Network Security
- Always use HTTPS for production deployments
- Implement proper CORS policies
- Consider IP restrictions for admin access

### 3. User Management
- Use Azure AD groups for role-based access control
- Regularly audit group memberships
- Implement just-in-time access for administrative functions

### 4. Monitoring
- Monitor failed authentication attempts
- Set up alerts for unusual login patterns
- Log OIDC events for compliance

## 🔄 Advanced Configuration

### Multi-Tenant Support
For supporting multiple Azure tenants:

```bash
# Use common endpoint for multi-tenant
GHOSTCRATE_OIDC_ENTRAID_AUTHORITY=https://login.microsoftonline.com/common
GHOSTCRATE_OIDC_ENTRAID_TENANT_ID=common
```

### Conditional Access
Configure Azure Conditional Access policies:
1. Go to **Azure AD** > **Security** > **Conditional Access**
2. Create policy for GhostCrate app
3. Configure conditions (location, device, risk level)
4. Set access controls (MFA, compliant devices)

### Custom Claims
Add custom user attributes:
1. Go to **Token configuration** in your app registration
2. Click **Add optional claim**
3. Select token type and claims
4. Configure in GhostCrate claim mappings

## 📞 Support

If you encounter issues:

1. Check GhostCrate logs: `docker-compose logs ghostcrate`
2. Verify Azure configuration matches this guide
3. Test with Azure AD Graph Explorer
4. Check [Microsoft Entra ID documentation](https://docs.microsoft.com/en-us/azure/active-directory/)

## 📋 Configuration Checklist

- [ ] Azure app registration created
- [ ] Client secret generated and stored securely
- [ ] API permissions configured and admin consent granted
- [ ] Redirect URI matches exactly
- [ ] Environment variables set in GhostCrate
- [ ] Nginx configuration updated (if using reverse proxy)
- [ ] GhostCrate restarted with new configuration
- [ ] OIDC login tested successfully
- [ ] Group-based access tested (if configured)
- [ ] Production security measures implemented

---

**🎉 Congratulations!** You've successfully configured Azure Entra ID OIDC for GhostCrate. Your users can now sign in using their corporate Microsoft accounts with full SSO integration.
