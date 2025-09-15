#!/bin/bash

# GhostCrate Production Deployment Script

set -e

echo "🚀 Deploying GhostCrate..."

# Check if environment file exists
if [ ! -f .env ]; then
    echo "❌ Error: .env file not found!"
    echo "📋 Please copy .env.example to .env and configure your settings:"
    echo "   cp .env.example .env"
    echo "   # Edit .env with your configuration"
    exit 1
fi

# Validate required environment variables
source .env

required_vars=(
    "GHOSTCRATE_AUTH_JWT_SECRET"
    "GHOSTCRATE_REGISTRY_BASE_URL"
    "DATABASE_URL"
)

for var in "${required_vars[@]}"; do
    if [ -z "${!var}" ]; then
        echo "❌ Error: Required environment variable $var is not set in .env"
        exit 1
    fi
done

# Check if JWT secret is the default placeholder
if [ "$GHOSTCRATE_AUTH_JWT_SECRET" = "your-super-secret-jwt-key-change-this-in-production" ]; then
    echo "❌ Error: Please change the GHOSTCRATE_AUTH_JWT_SECRET in .env file!"
    echo "💡 Generate a secure secret with: openssl rand -base64 32"
    exit 1
fi

echo "✅ Environment validation passed"

# Build and start the container
echo "🔨 Building Docker image..."
docker compose build

echo "🚀 Starting GhostCrate..."
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d

# Wait for the service to be ready
echo "⏳ Waiting for GhostCrate to be ready..."
for i in {1..30}; do
    if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
        echo "✅ GhostCrate is ready!"
        break
    fi
    echo "   Waiting... ($i/30)"
    sleep 2
done

if ! curl -sf http://localhost:8080/health > /dev/null 2>&1; then
    echo "❌ GhostCrate failed to start properly"
    echo "📋 Check logs with: docker compose logs ghostcrate"
    exit 1
fi

echo ""
echo "🎉 GhostCrate deployed successfully!"
echo ""
echo "📊 Service Status:"
docker compose ps
echo ""
echo "🔗 Local Access: http://localhost:8080"
echo "🔗 Health Check: http://localhost:8080/health"
echo ""
echo "📋 Useful commands:"
echo "  • View logs: docker compose logs -f ghostcrate"
echo "  • Stop service: docker compose down"
echo "  • Update: git pull && ./deploy.sh"
echo "  • Production deploy: docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d"
echo ""
echo "🔧 Next Steps for Production:"
echo "  1. Configure your nginx reverse proxy using nginx.conf.example"
echo "  2. Set up SSL certificates for your domain (Let's Encrypt recommended)"
echo "  3. Update DNS to point to your server"
echo "  4. Configure OAuth providers (GitHub, Azure AD, Google) in .env"
echo "  5. Test crate publishing: cargo publish --registry ghostcrate"
echo ""
echo "📖 Documentation: See DEPLOYMENT.md for detailed instructions"
echo ""