#!/bin/bash

# GhostCrate Proxmox LXC Deployment Script
# Usage: wget https://raw.githubusercontent.com/ghostkellz/ghostcrate/main/scripts/proxmox-lxc.sh -O - | bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 GhostCrate Proxmox LXC Deployment Script${NC}"
echo -e "${BLUE}=============================================${NC}"

# Check if running on Proxmox
if ! command -v pct &> /dev/null; then
    echo -e "${RED}❌ Error: This script must be run on a Proxmox VE host${NC}"
    exit 1
fi

# Step 1: Deploy Docker LXC using community script
echo -e "${YELLOW}📦 Deploying Docker LXC container using community script...${NC}"
echo -e "${BLUE}This will create a Docker-enabled LXC container${NC}"
echo ""

# Run the community Docker LXC script
bash -c "$(curl -fsSL https://raw.githubusercontent.com/community-scripts/ProxmoxVE/main/ct/docker.sh)"

# Check if the script completed successfully
if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Docker LXC deployment failed${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Docker LXC container created successfully${NC}"
echo ""

# Step 2: Get the container ID of the newly created container
echo -e "${YELLOW}🔍 Finding the newly created Docker container...${NC}"
CONTAINER_ID=$(pct list | grep -i docker | tail -1 | awk '{print $1}')

if [ -z "$CONTAINER_ID" ]; then
    echo -e "${RED}❌ Could not find the Docker container${NC}"
    echo "Please check the container list and run the GhostCrate setup manually:"
    echo "pct list"
    exit 1
fi

echo -e "${GREEN}✅ Found Docker container with ID: $CONTAINER_ID${NC}"

# Wait for container to be ready
echo -e "${YELLOW}⏳ Waiting for container to be ready...${NC}"
sleep 10

# Step 3: Setup GhostCrate in the Docker container
echo -e "${YELLOW}🚀 Setting up GhostCrate...${NC}"

# Create ghostcrate user
echo -e "${YELLOW}👤 Creating ghostcrate user...${NC}"
pct exec $CONTAINER_ID -- bash -c "
    useradd -m -s /bin/bash ghostcrate
    usermod -aG docker ghostcrate
    mkdir -p /home/ghostcrate
    chown ghostcrate:ghostcrate /home/ghostcrate
"

# Clone GhostCrate repository
echo -e "${YELLOW}📥 Cloning GhostCrate repository...${NC}"
pct exec $CONTAINER_ID -- bash -c "
    cd /home/ghostcrate
    git clone https://github.com/ghostkellz/ghostcrate.git || echo 'Repository not yet available - you can clone it manually later'
    chown -R ghostcrate:ghostcrate /home/ghostcrate/
"

# Create environment file
echo -e "${YELLOW}⚙️ Setting up configuration...${NC}"
pct exec $CONTAINER_ID -- bash -c "
    cd /home/ghostcrate/ghostcrate
    cp .env.example .env 2>/dev/null || echo 'DATABASE_URL=sqlite:/data/ghostcrate.db
LEPTOS_SITE_ADDR=0.0.0.0:8080
RUST_LOG=info
JWT_SECRET=change-this-in-production-\$(openssl rand -hex 32)
SESSION_DURATION_HOURS=168' > .env
    chown ghostcrate:ghostcrate .env
"

# Get container IP
CONTAINER_IP=$(pct exec $CONTAINER_ID -- hostname -I | awk '{print $1}')

echo -e "${GREEN}🎉 GhostCrate LXC container deployment completed!${NC}"
echo -e "${GREEN}=============================================${NC}"
echo ""
echo -e "${BLUE}📋 Container Details:${NC}"
echo "  Container ID: $CONTAINER_ID"
echo "  IP Address: $CONTAINER_IP"
echo "  SSH: ssh root@$CONTAINER_IP"
echo ""
echo -e "${BLUE}🚀 Next Steps:${NC}"
echo "1. SSH into the container: ssh root@$CONTAINER_IP"
echo "2. Switch to ghostcrate user: su - ghostcrate"
echo "3. Navigate to project: cd ~/ghostcrate"
echo "4. Start GhostCrate: docker-compose up -d"
echo "5. Access the web UI: http://$CONTAINER_IP:8080"
echo ""
echo -e "${YELLOW}⚠️  Important: Change the JWT_SECRET in .env before production use!${NC}"
echo ""
echo -e "${GREEN}✅ Deployment completed successfully!${NC}"