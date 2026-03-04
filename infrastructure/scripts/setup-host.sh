#!/usr/bin/env bash
#
# HomeLab Server Setup Script
# Run on a fresh Ubuntu Server 24.04 LTS install.
# Usage: ssh jacob@homelab 'bash -s' < setup-host.sh
#
set -euo pipefail

# ─── Configuration ──────────────────────────────────────────────────────────
HOSTNAME="homelab"
TIMEZONE="America/Chicago"
STATIC_IP="192.168.1.100/24"
GATEWAY="192.168.1.254"
DNS_SERVERS="1.1.1.1,8.8.8.8"
WIFI_INTERFACE="wlo1"         # check with: ip link show
HOMELAB_DIR="/opt/homelab"

# ─── Colors ─────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log()  { echo -e "${GREEN}[✓]${NC} $1"; }
warn() { echo -e "${YELLOW}[!]${NC} $1"; }
err()  { echo -e "${RED}[✗]${NC} $1"; exit 1; }

# ─── Preflight ──────────────────────────────────────────────────────────────
if [[ $EUID -ne 0 ]]; then
    err "Run this script as root: sudo bash setup-host.sh"
fi

echo ""
echo "══════════════════════════════════════════════════"
echo "  HomeLab Server Setup"
echo "  Ubuntu 24.04 LTS"
echo "══════════════════════════════════════════════════"
echo ""

# ─── 1. System basics ──────────────────────────────────────────────────────
log "Setting hostname to ${HOSTNAME}..."
hostnamectl set-hostname "$HOSTNAME"

log "Setting timezone to ${TIMEZONE}..."
timedatectl set-timezone "$TIMEZONE"

log "Updating package lists..."
apt-get update -qq

log "Upgrading packages..."
apt-get upgrade -y -qq

log "Installing essential packages..."
apt-get install -y -qq \
    git \
    curl \
    wget \
    htop \
    ufw \
    fail2ban \
    ca-certificates \
    gnupg \
    lsb-release \
    unattended-upgrades

# ─── 2. Disable lid close suspend ──────────────────────────────────────────
log "Disabling lid-close suspend (this is a server now)..."
sed -i 's/#HandleLidSwitch=suspend/HandleLidSwitch=ignore/' /etc/systemd/logind.conf
sed -i 's/#HandleLidSwitchExternalPower=suspend/HandleLidSwitchExternalPower=ignore/' /etc/systemd/logind.conf
sed -i 's/#HandleLidSwitchDocked=ignore/HandleLidSwitchDocked=ignore/' /etc/systemd/logind.conf
systemctl restart systemd-logind

# ─── 3. Firewall ───────────────────────────────────────────────────────────
log "Configuring firewall..."
ufw --force reset > /dev/null 2>&1
ufw default deny incoming
ufw default allow outgoing
ufw allow 22/tcp comment "SSH"
ufw allow 80/tcp comment "HTTP"
ufw allow 443/tcp comment "HTTPS"
ufw --force enable
log "Firewall enabled: SSH(22), HTTP(80), HTTPS(443)"

# ─── 4. Docker ─────────────────────────────────────────────────────────────
if command -v docker &> /dev/null; then
    warn "Docker already installed, skipping..."
else
    log "Installing Docker Engine..."

    # Remove old versions
    apt-get remove -y -qq docker docker-engine docker.io containerd runc 2>/dev/null || true

    # Add Docker GPG key
    install -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
    chmod a+r /etc/apt/keyrings/docker.gpg

    # Add Docker repo
    echo \
        "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
        $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null

    apt-get update -qq
    apt-get install -y -qq \
        docker-ce \
        docker-ce-cli \
        containerd.io \
        docker-buildx-plugin \
        docker-compose-plugin

    log "Docker installed: $(docker --version)"
fi

# Add the main user to docker group
MAIN_USER="${SUDO_USER:-jacob}"
if id "$MAIN_USER" &>/dev/null; then
    usermod -aG docker "$MAIN_USER"
    log "Added ${MAIN_USER} to docker group"
fi

# Enable Docker on boot
systemctl enable docker
systemctl start docker

# ─── 4b. Docker daemon config (log rotation + live-restore) ──────────────
log "Configuring Docker daemon (log rotation, live-restore)..."
cat > /etc/docker/daemon.json << 'EOF'
{
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "10m",
    "max-file": "3"
  },
  "live-restore": true
}
EOF
systemctl restart docker
log "Docker daemon configured: log rotation 10m/3 files, live-restore enabled"

# ─── 5. Create paas user for git push ──────────────────────────────────────
if id "paas" &>/dev/null; then
    warn "User 'paas' already exists, skipping..."
else
    log "Creating 'paas' user for git push access..."
    adduser --disabled-password --gecos "" paas
    usermod -aG docker paas

    # Copy SSH authorized keys from main user
    mkdir -p /home/paas/.ssh
    if [[ -f "/home/${MAIN_USER}/.ssh/authorized_keys" ]]; then
        cp "/home/${MAIN_USER}/.ssh/authorized_keys" /home/paas/.ssh/authorized_keys
    fi
    chown -R paas:paas /home/paas/.ssh
    chmod 700 /home/paas/.ssh
    chmod 600 /home/paas/.ssh/authorized_keys 2>/dev/null || true
    log "User 'paas' created with SSH key access"
fi

# ─── 6. Project directories ────────────────────────────────────────────────
log "Creating project directories..."
mkdir -p "${HOMELAB_DIR}"
mkdir -p "${HOMELAB_DIR}/git-repos"
mkdir -p "${HOMELAB_DIR}/data"
chown "${MAIN_USER}:${MAIN_USER}" "${HOMELAB_DIR}"
# git-repos and data must be writable by the API container (runs as paas, uid 1001)
chown paas:paas "${HOMELAB_DIR}/git-repos"
chown paas:paas "${HOMELAB_DIR}/data"
log "Created ${HOMELAB_DIR}/{git-repos,data} (owned by paas for container access)"

# ─── 7. SSH hardening ──────────────────────────────────────────────────────
log "Hardening SSH..."
# Use sshd_config.d drop-in (01 = loads before cloud-init's 50-cloud-init.conf).
# In OpenSSH, first match wins, so our settings take precedence.
cat > /etc/ssh/sshd_config.d/01-hardening.conf << 'EOF'
# HomeLab SSH Hardening — overrides cloud-init defaults
PasswordAuthentication no
PermitRootLogin no
PubkeyAuthentication yes
MaxAuthTries 3
LoginGraceTime 30
X11Forwarding no
AllowTcpForwarding no
ClientAliveInterval 300
ClientAliveCountMax 2
PermitEmptyPasswords no
EOF
# Remove cloud-init SSH override if present (it sets PasswordAuthentication yes)
rm -f /etc/ssh/sshd_config.d/50-cloud-init.conf
systemctl restart ssh
log "SSH: key-only auth, no root login, MaxAuthTries 3, idle timeout 5m"

# ─── 8. Auto security updates ──────────────────────────────────────────────
log "Enabling automatic security updates..."
cat > /etc/apt/apt.conf.d/20auto-upgrades << 'EOF'
APT::Periodic::Update-Package-Lists "1";
APT::Periodic::Unattended-Upgrade "1";
APT::Periodic::AutocleanInterval "7";
EOF

cat > /etc/apt/apt.conf.d/51unattended-upgrades-homelab << 'EOF'
// HomeLab: auto-reboot after kernel updates and clean up unused packages
Unattended-Upgrade::Automatic-Reboot "true";
Unattended-Upgrade::Automatic-Reboot-Time "03:00";
Unattended-Upgrade::Remove-Unused-Kernel-Packages "true";
Unattended-Upgrade::Remove-Unused-Dependencies "true";
EOF
log "Auto-reboot at 03:00 after kernel updates, unused packages cleaned"

# ─── 9. Kernel tuning ──────────────────────────────────────────────────────
log "Tuning kernel parameters..."
cat > /etc/sysctl.d/99-homelab.conf << 'EOF'
# HomeLab server tuning
vm.swappiness=10
kernel.panic=10
kernel.panic_on_oops=1
EOF
sysctl -p /etc/sysctl.d/99-homelab.conf > /dev/null
log "Kernel: swappiness=10, auto-reboot on panic after 10s"

# ─── 10. journald size limit ──────────────────────────────────────────────
log "Configuring journald size limits..."
sed -i '/^\[Journal\]$/a SystemMaxUse=200M\nSystemMaxFileSize=50M' /etc/systemd/journald.conf
systemctl restart systemd-journald
log "journald: max 200M total, 50M per file"

# ─── 11. Docker network ───────────────────────────────────────────────────
log "Creating homelab Docker network..."
docker network create homelab 2>/dev/null || warn "Network 'homelab' already exists"

# ─── 12. Static IP (applied last — may disconnect SSH) ─────────────────────
log "Configuring static IP: ${STATIC_IP}..."

# Detect active WiFi interface and SSID
# Try iwgetid first (available on most systems), fall back to iw
ACTIVE_WIFI_IFACE=""
CURRENT_SSID=""

for iface in $(ip -o link show | awk -F': ' '{print $2}' | grep -E '^wl'); do
    # Check if this WiFi interface is UP and has an IP
    if ip addr show "$iface" 2>/dev/null | grep -q 'inet '; then
        ACTIVE_WIFI_IFACE="$iface"
        # Try iwgetid for SSID
        CURRENT_SSID=$(iwgetid -r "$iface" 2>/dev/null || true)
        # Fall back to iw
        if [[ -z "$CURRENT_SSID" ]]; then
            CURRENT_SSID=$(iw dev "$iface" info 2>/dev/null | awk '/ssid/{print $2}' || true)
        fi
        break
    fi
done

if [[ -n "$ACTIVE_WIFI_IFACE" ]]; then
    log "Detected WiFi interface: ${ACTIVE_WIFI_IFACE}"
    if [[ -n "$CURRENT_SSID" ]]; then
        log "Connected to WiFi network: ${CURRENT_SSID}"
    fi

    # Try to extract existing WiFi password from current netplan config
    EXISTING_PASSWORD=""
    for f in /etc/netplan/*.yaml; do
        if [[ -f "$f" ]]; then
            pwd_line=$(grep -A1 'password:' "$f" 2>/dev/null | grep 'password:' | head -1 || true)
            if [[ -n "$pwd_line" ]]; then
                EXISTING_PASSWORD=$(echo "$pwd_line" | sed 's/.*password:[[:space:]]*"\?\([^"]*\)"\?/\1/')
                break
            fi
        fi
    done

    if [[ -n "$EXISTING_PASSWORD" && -n "$CURRENT_SSID" ]]; then
        cat > /etc/netplan/01-homelab.yaml << NETPLAN
network:
  version: 2
  wifis:
    ${ACTIVE_WIFI_IFACE}:
      dhcp4: no
      addresses:
        - ${STATIC_IP}
      routes:
        - to: default
          via: ${GATEWAY}
      nameservers:
        addresses: [${DNS_SERVERS}]
      access-points:
        "${CURRENT_SSID}":
          auth:
            key-management: psk
            password: "${EXISTING_PASSWORD}"
NETPLAN
        chmod 600 /etc/netplan/01-homelab.yaml
        log "Static IP config written with WiFi credentials from existing config"
        warn "Apply with: sudo netplan apply"
        warn "WARNING: Applying will change the IP and may disconnect SSH!"
    else
        WIFI_PASSWORD_PLACEHOLDER="YOUR_WIFI_PASSWORD"
        SSID_PLACEHOLDER="${CURRENT_SSID:-YOUR_WIFI_SSID}"
        cat > /etc/netplan/01-homelab.yaml << NETPLAN
network:
  version: 2
  wifis:
    ${ACTIVE_WIFI_IFACE}:
      dhcp4: no
      addresses:
        - ${STATIC_IP}
      routes:
        - to: default
          via: ${GATEWAY}
      nameservers:
        addresses: [${DNS_SERVERS}]
      access-points:
        "${SSID_PLACEHOLDER}":
          auth:
            key-management: psk
            password: "${WIFI_PASSWORD_PLACEHOLDER}"
NETPLAN
        chmod 600 /etc/netplan/01-homelab.yaml
        warn "Static IP config written but WiFi password needs to be set manually."
        warn "Edit /etc/netplan/01-homelab.yaml and set your SSID and password"
    fi
else
    # Wired connection — filter out virtual interfaces (docker, veth, br-, virbr)
    WIRED_IFACE=$(ip -o link show | awk -F': ' '{print $2}' | grep -v -E '^(lo|wl|docker|veth|br-|virbr)' | head -1)
    if [[ -n "$WIRED_IFACE" ]]; then
        cat > /etc/netplan/01-homelab.yaml << NETPLAN
network:
  version: 2
  ethernets:
    ${WIRED_IFACE}:
      dhcp4: no
      addresses:
        - ${STATIC_IP}
      routes:
        - to: default
          via: ${GATEWAY}
      nameservers:
        addresses: [${DNS_SERVERS}]
NETPLAN
        chmod 600 /etc/netplan/01-homelab.yaml
        warn "Static IP config written for wired interface: ${WIRED_IFACE}"
        warn "Apply with: sudo netplan apply"
        warn "WARNING: Applying will change the IP and may disconnect SSH!"
    else
        warn "No network interface detected for static IP configuration."
        warn "Configure /etc/netplan/01-homelab.yaml manually."
    fi
fi

# ─── Done ───────────────────────────────────────────────────────────────────
echo ""
echo "══════════════════════════════════════════════════"
echo "  Setup complete!"
echo "══════════════════════════════════════════════════"
echo ""
log "Hostname:     ${HOSTNAME}"
log "Timezone:     ${TIMEZONE}"
log "Docker:       $(docker --version 2>/dev/null || echo 'not installed')"
log "Firewall:     active (22, 80, 443)"
log "SSH:          key-only, MaxAuthTries 3, idle timeout 5m"
log "fail2ban:     active (SSH protection)"
log "Docker logs:  10m/3 files, live-restore on"
log "Kernel:       swappiness=10, panic reboot=10s"
log "journald:     max 200M"
log "Auto-updates: security patches + reboot at 03:00"
log "Lid suspend:  disabled"
log "Project dir:  ${HOMELAB_DIR}"
echo ""
warn "Next steps:"
echo "  1. Set WiFi password in /etc/netplan/01-homelab.yaml (if on WiFi)"
echo "  2. Apply static IP: sudo netplan apply"
echo "  3. Reconnect SSH at the new IP: ssh jacob@192.168.1.100"
echo "  4. Log out and back in for docker group to take effect"
echo "  5. Clone the repo: cd /opt/homelab && git clone <your-repo-url> ."
echo ""
