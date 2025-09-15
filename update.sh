#!/bin/bash

# GhostCrate Update Script
# Safely updates GhostCrate with backup and rollback capability

set -e

echo "🔄 Updating GhostCrate..."

# Check if we're in a git repository
if [ ! -d .git ]; then
    echo "❌ Error: Not in a git repository"
    exit 1
fi

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    echo "⚠️  Warning: You have uncommitted changes"
    echo "   Commit or stash them before updating"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Store current commit for rollback
CURRENT_COMMIT=$(git rev-parse HEAD)
echo "📝 Current commit: $CURRENT_COMMIT"

# Backup current .env if it exists
if [ -f .env ]; then
    cp .env .env.backup.$(date +%s)
    echo "💾 Backed up .env file"
fi

# Pull latest changes
echo "📥 Pulling latest changes..."
git fetch origin
git pull origin main

# Check if there are new commits
NEW_COMMIT=$(git rev-parse HEAD)
if [ "$CURRENT_COMMIT" = "$NEW_COMMIT" ]; then
    echo "✅ Already up to date!"
    exit 0
fi

echo "🆕 Updated to commit: $NEW_COMMIT"

# Check if .env.example was updated and warn user
if git diff --name-only $CURRENT_COMMIT $NEW_COMMIT | grep -q ".env.example"; then
    echo "⚠️  .env.example was updated - you may need to update your .env file"
    echo "   Compare with: diff .env .env.example"
fi

# Stop existing containers
echo "🛑 Stopping existing containers..."
docker compose down

# Rebuild images
echo "🔨 Rebuilding Docker images..."
if ! docker compose build; then
    echo "❌ Build failed! Rolling back..."
    git reset --hard $CURRENT_COMMIT
    echo "🔄 Rolled back to previous commit"
    exit 1
fi

# Start updated containers
echo "🚀 Starting updated containers..."
if ! docker compose up -d; then
    echo "❌ Failed to start containers! Rolling back..."
    git reset --hard $CURRENT_COMMIT
    docker compose build
    docker compose up -d
    echo "🔄 Rolled back to previous commit"
    exit 1
fi

# Wait for health check
echo "⏳ Waiting for health check..."
for i in {1..30}; do
    if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
        echo "✅ Update successful!"
        break
    fi
    echo "   Waiting... ($i/30)"
    sleep 2
done

if ! curl -sf http://localhost:8080/health > /dev/null 2>&1; then
    echo "❌ Health check failed! Rolling back..."
    docker compose down
    git reset --hard $CURRENT_COMMIT
    docker compose build
    docker compose up -d
    echo "🔄 Rolled back to previous commit"
    exit 1
fi

echo ""
echo "🎉 GhostCrate updated successfully!"
echo "📊 Service Status:"
docker compose ps
echo ""
echo "📋 Changes:"
git log --oneline $CURRENT_COMMIT..$NEW_COMMIT
echo ""