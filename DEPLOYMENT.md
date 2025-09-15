# GhostCrate Production Deployment Guide

This guide covers deploying GhostCrate v0.2.0 behind nginx for public hosting.

## 🚀 Quick Start

1. **Configure Environment**:
   ```bash
   cp .env.example .env
   # Edit .env with your production settings
   ```

2. **Deploy**:
   ```bash
   chmod +x deploy.sh
   ./deploy.sh
   ```

## 📁 Files Overview

### Core Configuration
- `docker-compose.yml` - Base Docker Compose configuration
- `docker-compose.prod.yml` - Production overrides with security and resource limits
- `Dockerfile` - Optimized multi-stage build using Rust nightly for edition 2024 support

### Templates & Examples
- `.env.example` - Comprehensive environment variable template
- `nginx.conf.example` - Production-ready nginx reverse proxy configuration with Let's Encrypt
- `cargo-config.toml.example` - Cargo configuration for using your private registry
- `deploy.sh` - Automated deployment script with validation
- `update.sh` - Safe update script with rollback capability

## 🔧 Configuration

### Domain Setup
Update these environment variables in `.env`:
```env
GHOSTCRATE_REGISTRY_BASE_URL=https://crates.cktechx.com
GHOSTCRATE_REGISTRY_INDEX_URL=https://crates.cktechx.com
GHOSTCRATE_CORS_ORIGINS=https://crates.cktechx.com,https://crates.cktechnology.io,https://crates.cktechx.io
```

### Security
- Generate a secure JWT secret: `openssl rand -base64 32`
- Configure CORS origins for your domains
- Set appropriate rate limits
- Use HTTPS in production

### OAuth/OIDC Integration
The deployment supports multiple authentication providers:
- GitHub OAuth
- Microsoft Entra ID (Azure AD)
- Google OAuth
- GitHub OIDC

## 🔧 Nginx Configuration

1. Copy `nginx.conf.example` to your nginx sites-available directory
2. Update SSL certificate paths
3. Adjust domain names
4. Enable the site and reload nginx

## 🐳 Docker Deployment

### Development
```bash
docker compose up -d
```

### Production
```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d
```

## 📊 Monitoring & Health Checks

- Health endpoint: `/health`
- Metrics endpoint: `/metrics` (if enabled)
- Built-in health checks with restart policies
- Resource limits in production mode

## 🔄 Maintenance

### Updates
```bash
# Safe update with rollback capability
./update.sh

# Or manual update
git pull
./deploy.sh
```

### Logs
```bash
docker compose logs -f ghostcrate
```

### Backup
The SQLite database and uploaded crates are stored in the `ghostcrate_data` Docker volume.

## 🎯 Production Optimizations

- Multi-stage Docker build for smaller image size
- Debian sid-slim for glibc compatibility with Rust nightly
- Non-root user execution
- Resource limits and health checks
- Proper volume permissions
- Security headers in nginx
- TLS/SSL configuration
- CORS and rate limiting

## 🔐 Security Features

- JWT-based authentication
- bcrypt password hashing
- CORS protection
- Rate limiting
- Security headers via nginx
- Non-root container execution
- Proper file permissions

## 📋 Troubleshooting

### Container Won't Start
- Check logs: `docker compose logs ghostcrate`
- Verify environment variables in `.env`
- Ensure proper file permissions

### Database Issues
- Database is stored in `/data` inside container
- Mapped to Docker volume `ghostcrate_data`
- Check volume permissions

### Build Issues
- Uses Rust nightly for edition 2024 support
- Requires compatible glibc (Debian sid-slim)
- Network access needed for dependency downloads

## 🌐 Supported Domains

The configuration supports multiple domain aliases:
- `crates.cktechx.com` (primary)
- `crates.cktechnology.io`
- `crates.cktechx.io`

All redirect URLs and CORS origins are configured for these domains.

---

For additional help, check the project README or submit issues to the repository.