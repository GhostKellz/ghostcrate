#!/bin/bash

# GhostCrate Proxmox LXC Deployment Script
# Usage: wget https://raw.githubusercontent.com/ghostkellz/ghostcrate/main/scripts/proxmox-lxc.sh -O - | bash

set -e

# Configuration
CONTAINER_ID=${CONTAINER_ID:-200}
CONTAINER_NAME="ghostcrate"
TEMPLATE="ubuntu-22.04-standard"
DISK_SIZE="8G"
MEMORY="2048"
CORES="2"
NETWORK="vmbr0"
STORAGE="local-lxc"

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

echo -e "${YELLOW}📋 Configuration:${NC}"
echo "  Container ID: $CONTAINER_ID"
echo "  Container Name: $CONTAINER_NAME"
echo "  Template: $TEMPLATE"
echo "  Disk Size: $DISK_SIZE"
echo "  Memory: ${MEMORY}MB"
echo "  CPU Cores: $CORES"
echo ""

# Check if container ID is already in use
if pct status $CONTAINER_ID &> /dev/null; then
    echo -e "${RED}❌ Error: Container ID $CONTAINER_ID is already in use${NC}"
    echo "Available container IDs:"
    pvesh get /cluster/nextid
    exit 1
fi

# Create the LXC container
echo -e "${YELLOW}📦 Creating LXC container...${NC}"
pct create $CONTAINER_ID local:vztmpl/${TEMPLATE}_amd64.tar.xz \
    --hostname $CONTAINER_NAME \
    --memory $MEMORY \
    --cores $CORES \
    --rootfs ${STORAGE}:${DISK_SIZE} \
    --net0 name=eth0,bridge=${NETWORK},ip=dhcp \
    --features nesting=1 \
    --unprivileged 1 \
    --start 1

echo -e "${GREEN}✅ Container created and started${NC}"

# Wait for container to fully boot
echo -e "${YELLOW}⏳ Waiting for container to boot...${NC}"
sleep 30

# Update the container
echo -e "${YELLOW}📦 Updating container packages...${NC}"
pct exec $CONTAINER_ID -- bash -c "apt update && apt upgrade -y"

# Install Docker
echo -e "${YELLOW}🐳 Installing Docker...${NC}"
pct exec $CONTAINER_ID -- bash -c "
    apt install -y ca-certificates curl gnupg lsb-release
    mkdir -p /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
    echo \"deb [arch=\$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \$(lsb_release -cs) stable\" | tee /etc/apt/sources.list.d/docker.list > /dev/null
    apt update
    apt install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
    systemctl enable docker
    systemctl start docker
"

# Install Docker Compose (standalone)
echo -e "${YELLOW}🔧 Installing Docker Compose...${NC}"
pct exec $CONTAINER_ID -- bash -c "
    curl -L \"https://github.com/docker/compose/releases/latest/download/docker-compose-\$(uname -s)-\$(uname -m)\" -o /usr/local/bin/docker-compose
    chmod +x /usr/local/bin/docker-compose
"

# Create ghostcrate user
echo -e "${YELLOW}👤 Creating ghostcrate user...${NC}"
pct exec $CONTAINER_ID -- bash -c "
    useradd -m -s /bin/bash ghostcrate
    usermod -aG docker ghostcrate
    mkdir -p /home/ghostcrate/ghostcrate
    chown ghostcrate:ghostcrate /home/ghostcrate/ghostcrate
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
    cp .env.example .env || echo 'DATABASE_URL=sqlite:/data/ghostcrate.db
LEPTOS_SITE_ADDR=0.0.0.0:8080
RUST_LOG=info
JWT_SECRET=change-this-in-production-$(openssl rand -hex 32)
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
echo "  Container Name: $CONTAINER_NAME"
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