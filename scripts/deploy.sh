#!/bin/bash
# GhostCrate Production Deployment Script

set -euo pipefail

echo "🚀 Deploying GhostCrate to production..."

# Configuration
DOMAIN="${DOMAIN:-crates.yourdomain.com}"
DATA_DIR="${DATA_DIR:-/opt/ghostcrate/data}"
COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.yml}"

# Create data directory
echo "📁 Creating data directory..."
sudo mkdir -p "$DATA_DIR"
sudo chown 1001:1001 "$DATA_DIR"

# Generate secure JWT secret if not provided
if [ -z "${GHOSTCRATE_AUTH_JWT_SECRET:-}" ]; then
    echo "🔐 Generating secure JWT secret..."
    export GHOSTCRATE_AUTH_JWT_SECRET=$(openssl rand -base64 32)
    echo "Generated JWT secret: $GHOSTCRATE_AUTH_JWT_SECRET"
    echo "⚠️  SAVE THIS SECRET - YOU'LL NEED IT FOR FUTURE DEPLOYMENTS"
fi

# Pull latest images
echo "📦 Pulling latest Docker images..."
docker-compose -f "$COMPOSE_FILE" pull

# Build and start services
echo "🏗️  Building and starting services..."
docker-compose -f "$COMPOSE_FILE" up --build -d

# Wait for health check
echo "🏥 Waiting for health check..."
for i in {1..30}; do
    if curl -f http://localhost:8080/health >/dev/null 2>&1; then
        echo "✅ GhostCrate is healthy!"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "❌ Health check failed after 30 attempts"
        docker-compose -f "$COMPOSE_FILE" logs ghostcrate
        exit 1
    fi
    echo "⏳ Attempt $i/30..."
    sleep 2
done

# Show status
echo "📊 Service status:"
docker-compose -f "$COMPOSE_FILE" ps

echo ""
echo "🎉 GhostCrate deployed successfully!"
echo "🌐 Access your registry at: https://$DOMAIN"
echo "🔧 Admin dashboard: https://$DOMAIN/admin"
echo "📊 Health check: https://$DOMAIN/health"
echo ""
echo "📚 Next steps:"
echo "1. Configure your cargo config: ~/.cargo/config.toml"
echo "   [registries]"
echo "   ghostcrate = { index = \"https://$DOMAIN\" }"
echo ""
echo "2. Register a user account at: https://$DOMAIN"
echo "3. Generate an API token in your profile"
echo "4. Publish crates with: cargo publish --registry ghostcrate"
echo ""

# Optional: Setup nginx if requested
if [ "${SETUP_NGINX:-false}" = "true" ]; then
    echo "🌐 Setting up Nginx..."
    sudo cp nginx.conf "/etc/nginx/sites-available/ghostcrate"
    sudo sed -i "s/crates.yourdomain.com/$DOMAIN/g" "/etc/nginx/sites-available/ghostcrate"
    sudo ln -sf "/etc/nginx/sites-available/ghostcrate" "/etc/nginx/sites-enabled/"
    sudo nginx -t && sudo systemctl reload nginx
    echo "✅ Nginx configured"
fi

# Optional: Setup SSL with certbot
if [ "${SETUP_SSL:-false}" = "true" ]; then
    echo "🔒 Setting up SSL certificate..."
    sudo certbot --nginx -d "$DOMAIN" --non-interactive --agree-tos --email "${EMAIL:-admin@$DOMAIN}"
    echo "✅ SSL certificate configured"
fi

echo "🏁 Deployment complete!"
