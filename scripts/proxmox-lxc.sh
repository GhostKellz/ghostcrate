#!/bin/bash

# GhostCrate Proxmox LXC Deployment Script
# Usage: wget https://raw.githubusercontent.com/ghostkellz/ghostcrate/main/scripts/proxmox-lxc.sh -O - | bash

set -e

# Helper functions
get_next_container_id() {
    local next_id
    next_id=$(pvesh get /cluster/nextid --format json | jq -r '.')
    echo "$next_id"
}

get_available_bridges() {
    ip link show | grep -E '^[0-9]+: vmbr[0-9]+' | sed 's/.*: \(vmbr[0-9]\+\).*/\1/' | sort
}

validate_ip() {
    local ip=$1
    if [[ $ip =~ ^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}$ ]]; then
        IFS='.' read -ra ADDR <<< "$ip"
        for i in "${ADDR[@]}"; do
            if [[ $i -gt 255 ]]; then
                return 1
            fi
        done
        return 0
    fi
    return 1
}

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

# Check for required tools
if ! command -v jq &> /dev/null; then
    echo -e "${YELLOW}📦 Installing jq...${NC}"
    apt update && apt install -y jq
fi

# Setup Mode Selection
echo -e "${YELLOW}⚙️ Setup Configuration${NC}"
echo ""
echo "Setup modes:"
echo "1. Default setup (quick deployment with recommended settings)"
echo "2. Advanced setup (customize all settings)"
echo ""
read -p "Choose setup mode (1 or 2, default: 1): " setup_mode
setup_mode=${setup_mode:-1}

if [ "$setup_mode" = "2" ]; then
    # Advanced Configuration
    echo ""
    echo -e "${BLUE}🔧 Advanced Configuration Mode${NC}"
    
    # Container ID selection
    echo ""
    echo -e "${BLUE}📋 Container ID Selection${NC}"
    suggested_id=$(get_next_container_id)
    echo "Suggested next available ID: $suggested_id"
    read -p "Enter container ID (or press Enter for $suggested_id): " user_container_id
    CONTAINER_ID=${user_container_id:-$suggested_id}

    # Validate container ID isn't in use
    if pct status $CONTAINER_ID &> /dev/null; then
        echo -e "${RED}❌ Error: Container ID $CONTAINER_ID is already in use${NC}"
        echo "Available container IDs:"
        pvesh get /cluster/nextid
        exit 1
    fi

    # Container name selection
    echo ""
    echo -e "${BLUE}📝 Container Name${NC}"
    read -p "Enter container name (default: ghostcrate): " user_container_name
    CONTAINER_NAME=${user_container_name:-"ghostcrate"}

    # Network bridge selection
    echo ""
    echo -e "${BLUE}🌐 Network Configuration${NC}"
    echo "Available network bridges:"
    available_bridges=$(get_available_bridges)
    if [ -z "$available_bridges" ]; then
        echo "  vmbr0 (default)"
        NETWORK="vmbr0"
    else
        echo "$available_bridges" | nl -w2 -s'. '
        echo ""
        read -p "Select network bridge (default: vmbr0): " user_network
        NETWORK=${user_network:-"vmbr0"}
    fi

    # IP Configuration
    echo ""
    echo -e "${BLUE}🔗 IP Address Configuration${NC}"
    echo "1. DHCP (automatic)"
    echo "2. Static IP"
    read -p "Choose IP configuration (1 or 2, default: 1): " ip_choice
    ip_choice=${ip_choice:-1}

    if [ "$ip_choice" = "2" ]; then
        while true; do
            read -p "Enter static IP address (e.g., 192.168.1.100/24): " static_ip
            if [[ $static_ip =~ ^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}/[0-9]{1,2}$ ]]; then
                IP_CONFIG="ip=$static_ip"
                break
            else
                echo -e "${RED}❌ Invalid IP format. Please use format: 192.168.1.100/24${NC}"
            fi
        done
        read -p "Enter gateway IP (optional): " gateway_ip
        if [ -n "$gateway_ip" ] && validate_ip "$gateway_ip"; then
            IP_CONFIG="$IP_CONFIG,gw=$gateway_ip"
        fi
    else
        IP_CONFIG="ip=dhcp"
    fi

    # Resource configuration
    echo ""
    echo -e "${BLUE}⚡ Resource Configuration${NC}"
    read -p "Memory in MB (default: 2048): " user_memory
    MEMORY=${user_memory:-2048}

    read -p "CPU cores (default: 2): " user_cores
    CORES=${user_cores:-2}

    read -p "Disk size (default: 8G): " user_disk
    DISK_SIZE=${user_disk:-"8G"}

    # Storage selection
    read -p "Storage location (default: local-lxc): " user_storage
    STORAGE=${user_storage:-"local-lxc"}

    # Template selection
    read -p "Template (default: ubuntu-22.04-standard): " user_template
    TEMPLATE=${user_template:-"ubuntu-22.04-standard"}
else
    # Default Configuration
    echo ""
    echo -e "${BLUE}⚡ Default Configuration Mode${NC}"
    CONTAINER_ID=$(get_next_container_id)
    CONTAINER_NAME="ghostcrate"
    TEMPLATE="ubuntu-22.04-standard"
    DISK_SIZE="8G"
    MEMORY="2048"
    CORES="2"
    NETWORK="vmbr0"
    STORAGE="local-lxc"
    IP_CONFIG="ip=dhcp"
    
    # Validate container ID isn't in use
    if pct status $CONTAINER_ID &> /dev/null; then
        echo -e "${RED}❌ Error: Container ID $CONTAINER_ID is already in use${NC}"
        echo "Available container IDs:"
        pvesh get /cluster/nextid
        exit 1
    fi
fi

echo ""
echo -e "${YELLOW}📋 Final Configuration:${NC}"
echo "  Container ID: $CONTAINER_ID"
echo "  Container Name: $CONTAINER_NAME"
echo "  Template: $TEMPLATE"
echo "  Disk Size: $DISK_SIZE"
echo "  Memory: ${MEMORY}MB"
echo "  CPU Cores: $CORES"
echo "  Network Bridge: $NETWORK"
echo "  IP Configuration: $IP_CONFIG"
echo "  Storage: $STORAGE"
echo ""

read -p "Proceed with container creation? (y/N): " confirm
if [[ ! $confirm =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}⚠️ Container creation cancelled${NC}"
    exit 0
fi

# Create the LXC container
echo -e "${YELLOW}📦 Creating LXC container...${NC}"
pct create $CONTAINER_ID local:vztmpl/${TEMPLATE}_amd64.tar.xz \
    --hostname $CONTAINER_NAME \
    --memory $MEMORY \
    --cores $CORES \
    --rootfs ${STORAGE}:${DISK_SIZE} \
    --net0 name=eth0,bridge=${NETWORK},$IP_CONFIG \
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