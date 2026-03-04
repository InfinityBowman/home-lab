# Laptop Setup Guide

Step-by-step guide to turn the HP laptop into a home lab server.

## Hardware Specs

| Component | Details |
|-----------|---------|
| **Model** | HP Spectre x360 Convertible 15t-eb000 |
| **CPU** | Intel Core i7-10750H @ 2.60GHz (6 cores / 12 threads) |
| **RAM** | 16 GB |
| **Storage** | 954 GB NVMe SSD (Intel HBRPEKNX0203AH) + 27 GB NVMe (Intel Optane cache) |
| **GPU** | NVIDIA GeForce GTX 1650 Ti Mobile + Intel UHD Graphics (CometLake-H) |
| **OS** | Ubuntu 24.04.4 LTS |
| **Docker** | Docker 29.2.1, Compose v5.1.0 |
| **Root Partition** | 98 GB (86 GB available) |
| **Swap** | 4 GB |

### Resource Budget Estimate

Planning how much headroom you have for services:

| Service | RAM (est.) | Notes |
|---------|-----------|-------|
| Ubuntu OS + overhead | ~1 GB | Base system |
| PaaS (Traefik + API + deployed apps) | ~1-2 GB | Depends on deployed apps |
| n8n + PostgreSQL | ~1-2 GB | Lightweight for personal use |
| Twenty CRM (server + worker + PG + Redis) | ~4-6 GB | Heaviest service |
| OpenClaw or NanoClaw | ~1-2 GB | API-based, no local LLM |
| **Total estimate** | **~8-13 GB** | Tight if running everything simultaneously |

> **Note:** The GTX 1650 Ti (4 GB VRAM) could run small local LLMs via Ollama, but only tiny models (~3B parameters). For serious local LLM inference, a cloud GPU or dedicated machine would be better. The NVIDIA driver (`nvidia-smi`) is not currently installed — see GPU setup section below if needed.

## 1. Install Ubuntu Server

### Create bootable USB

On your Mac:
```bash
# Download Ubuntu Server 24.04 LTS
# https://ubuntu.com/download/server

# Find your USB drive
diskutil list

# Unmount it (replace diskN with your disk number)
diskutil unmountDisk /dev/diskN

# Write the ISO (use rdiskN for faster writes)
sudo dd if=~/Downloads/ubuntu-24.04-live-server-amd64.iso of=/dev/rdiskN bs=4m status=progress

# Eject
diskutil eject /dev/diskN
```

### Install Ubuntu

1. Plug USB into HP laptop, boot from USB (usually F9 or F12 for boot menu, F10 for BIOS)
2. Select "Install Ubuntu Server"
3. Language: English
4. Keyboard: your layout
5. Network: connect to WiFi or Ethernet
6. Proxy: skip
7. Mirror: default
8. Storage: **Use entire disk** (wipes everything)
9. Profile setup:
   - Name: whatever you want
   - Server name: `homelab` (this becomes the hostname)
   - Username: `jacob` (or whatever you prefer)
   - Password: something strong
10. SSH: **Install OpenSSH server** (check this box!)
11. Featured snaps: skip all
12. Wait for install, reboot, remove USB

### First boot

After reboot, log in at the console and note the IP address:
```bash
ip addr show
# Look for something like 192.168.1.XXX on your LAN interface
```

From your Mac, you should now be able to SSH in:
```bash
ssh jacob@192.168.1.XXX
```

## 2. Initial Server Configuration

### Set a static IP (recommended)

You don't want the laptop's IP changing. Edit netplan:
```bash
sudo nano /etc/netplan/00-installer-config.yaml
```

For WiFi:
```yaml
network:
  version: 2
  wifis:
    wlp2s0:  # your interface name from `ip addr`
      dhcp4: no
      addresses:
        - 192.168.1.100/24   # pick an IP outside your router's DHCP range
      routes:
        - to: default
          via: 192.168.1.1    # your router's IP
      nameservers:
        addresses: [1.1.1.1, 8.8.8.8]
      access-points:
        "YourWiFiName":
          password: "YourWiFiPassword"
```

For Ethernet:
```yaml
network:
  version: 2
  ethernets:
    enp1s0:  # your interface name
      dhcp4: no
      addresses:
        - 192.168.1.100/24
      routes:
        - to: default
          via: 192.168.1.1
      nameservers:
        addresses: [1.1.1.1, 8.8.8.8]
```

Apply:
```bash
sudo netplan apply
```

### Update everything
```bash
sudo apt update && sudo apt upgrade -y
```

### Set up firewall and fail2ban
```bash
sudo apt install -y fail2ban
sudo systemctl enable fail2ban

sudo ufw allow 22/tcp    # SSH
sudo ufw allow 80/tcp    # HTTP (Traefik)
sudo ufw allow 443/tcp   # HTTPS (Traefik)
sudo ufw enable
sudo ufw status
```

fail2ban watches SSH logs for repeated failed login attempts and automatically bans the offending IP.

### Disable laptop lid close suspend

Since this is a server, you don't want it sleeping when you close the lid:
```bash
sudo sed -i 's/#HandleLidSwitch=suspend/HandleLidSwitch=ignore/' /etc/systemd/logind.conf
sudo sed -i 's/#HandleLidSwitchExternalPower=suspend/HandleLidSwitchExternalPower=ignore/' /etc/systemd/logind.conf
sudo systemctl restart systemd-logind
```

### Set timezone
```bash
sudo timedatectl set-timezone America/Chicago  # adjust to your timezone
```

## 3. Install Docker

```bash
# Remove any old versions
sudo apt remove -y docker docker-engine docker.io containerd runc 2>/dev/null

# Install prerequisites
sudo apt install -y ca-certificates curl gnupg

# Add Docker GPG key
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
sudo chmod a+r /etc/apt/keyrings/docker.gpg

# Add Docker repo
echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
  $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
  sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

# Install Docker Engine + Compose
sudo apt update
sudo apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

# Add your user to the docker group (so you don't need sudo)
sudo usermod -aG docker $USER

# Log out and back in for group change to take effect
exit
# SSH back in

# Verify
docker run hello-world
docker compose version
```

## 4. Install Git and Other Tools

```bash
sudo apt install -y git curl wget htop
```

## 5. Create the PaaS User (for git push access)

```bash
# Create a 'paas' user with git-shell for security
sudo adduser --disabled-password --gecos "" paas
sudo usermod -aG docker paas

# Set up SSH key auth for the paas user
sudo mkdir -p /home/paas/.ssh
sudo touch /home/paas/.ssh/authorized_keys
sudo chown -R paas:paas /home/paas/.ssh
sudo chmod 700 /home/paas/.ssh
sudo chmod 600 /home/paas/.ssh/authorized_keys
```

Add your Mac's public key:
```bash
# On your Mac, copy your public key:
cat ~/.ssh/id_ed25519.pub  # or id_rsa.pub

# On the server, paste it into:
sudo nano /home/paas/.ssh/authorized_keys
```

## 6. Set Up Project Directories

```bash
sudo mkdir -p /opt/homelab
sudo chown $USER:$USER /opt/homelab

# Clone the repo (once you push it to GitHub)
cd /opt/homelab
git clone https://github.com/YOUR_USERNAME/home-lab.git .

# Or just create the directories manually for now
mkdir -p git-repos data
```

## 7. Set Up Environment Variables

```bash
cp .env.example .env
nano .env
```

Fill in:
```bash
# Cloudflare (set up tunnel first — see docs/cloudflare-tunnel-setup.md)
CLOUDFLARE_API_TOKEN=your_token_here
CLOUDFLARE_ACCOUNT_ID=your_account_id
CLOUDFLARE_TUNNEL_ID=your_tunnel_id
CLOUDFLARE_TUNNEL_TOKEN=your_tunnel_token
CLOUDFLARE_ZONE_ID=your_zone_id
BASE_DOMAIN=lab.yourdomain.com

# Security
INTERNAL_HOOK_SECRET=generate_a_random_string_here

# Database
DATABASE_URL=sqlite:///data/homelab.db
```

## 8. Start the PaaS

```bash
cd /opt/homelab/infrastructure
docker compose up -d

# Check everything is running
docker compose ps
docker compose logs -f
```

## 9. Auto-Start on Boot

Docker Compose services with `restart: unless-stopped` will auto-start when Docker starts. Docker itself starts on boot by default on Ubuntu.

Verify:
```bash
sudo systemctl is-enabled docker
# Should say "enabled"
```

### Automatic security updates and reboot

The setup script configures unattended-upgrades to auto-install security patches and auto-reboot at 3:00 AM when kernel updates require it. Verify:
```bash
cat /etc/apt/apt.conf.d/20auto-upgrades
cat /etc/apt/apt.conf.d/51unattended-upgrades-homelab
```

### Kernel tuning

The setup script configures `/etc/sysctl.d/99-homelab.conf` with:
- `vm.swappiness=10` — only swap under real memory pressure
- `kernel.panic=10` — auto-reboot after kernel panic (headless server would otherwise hang forever)

### Docker daemon config

`/etc/docker/daemon.json` is configured with:
- **Log rotation** — 10 MB max per log file, 3 files max per container (prevents unbounded disk usage)
- **Live-restore** — containers keep running during Docker daemon restarts (e.g., Docker updates)

### journald size limit

System logs are capped at 200 MB total via `/etc/systemd/journald.conf`.

## 10. Keeping the Laptop Happy as a Server

- **Power:** Keep it plugged in at all times
- **Lid:** Can be closed (we disabled suspend above)
- **Thermals:** Keep it on a hard surface with airflow, not on carpet/bed
- **Monitoring:** `htop` for CPU/RAM, `df -h` for disk, `docker stats` for container resources
- **Updates:** Security patches are auto-installed. The server will auto-reboot at 3:00 AM if a kernel update requires it. For non-security updates, run `sudo apt update && sudo apt upgrade -y` periodically
- **Backups:** The SQLite DB is in `/opt/homelab/data/`. Back it up occasionally:
  ```bash
  cp /opt/homelab/data/homelab.db /opt/homelab/data/homelab.db.bak.$(date +%Y%m%d)
  ```

## Quick Reference

| What | Command |
|------|---------|
| SSH in | `ssh jacob@192.168.1.100` |
| Check services | `cd /opt/homelab/infrastructure && docker compose ps` |
| View logs | `docker compose logs -f paas-api` |
| Restart everything | `docker compose restart` |
| Stop everything | `docker compose down` |
| Disk usage | `df -h` |
| Docker disk usage | `docker system df` |
| Clean old images | `docker image prune -a` |
| System resources | `htop` |
