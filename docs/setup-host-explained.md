# Server Setup Script — Explained

A beginner-friendly breakdown of everything `infrastructure/scripts/setup-host.sh` does to turn a fresh Ubuntu Server install into a home lab.

## What is this script?

It automates all the tedious first-time setup you'd do manually after installing Ubuntu Server. Instead of running 50+ commands by hand, you run one script and it handles everything.

```bash
ssh jacob@192.168.1.235 'sudo bash -s' < infrastructure/scripts/setup-host.sh
```

That command says: "SSH into the server, start a root shell, and feed this script into it."

---

## Step by Step

### 1. System Basics

**Set hostname:**
```bash
hostnamectl set-hostname homelab
```
The hostname is your machine's name on the network. When you open a terminal, you see `jacob@homelab`. Other devices on your network can find it by this name.

**Set timezone:**
```bash
timedatectl set-timezone America/Chicago
```
Servers default to UTC (London time). This sets it to your local time so log timestamps make sense.

**Update packages:**
```bash
apt-get update    # Refresh the list of available software
apt-get upgrade   # Install newer versions of everything
```
Like "Check for Updates" on your phone. Ubuntu ships with whatever was current when the ISO was built — this catches up to the latest patches.

**Install essential packages:**

| Package | What it does | Why we need it |
|---------|-------------|----------------|
| `git` | Version control | The deploy pipeline is built on git. Every app is a git repo. |
| `curl` | Make HTTP requests from the terminal | The git hooks use curl to notify the PaaS API when code is pushed |
| `wget` | Download files from the internet | General utility for grabbing things |
| `htop` | Interactive system monitor | Like Task Manager/Activity Monitor but for Linux. Shows CPU, RAM, and running processes in a nice visual layout. Run `htop` anytime to see what your server is doing. |
| `ufw` | Firewall manager | Controls which network ports are open. Blocks everything except what we explicitly allow. |
| `ca-certificates` | Trusted SSL certificates | Without these, your server can't verify HTTPS connections. Needed for Docker image pulls, Cloudflare API calls, etc. |
| `gnupg` | Encryption/signing tool | Used to verify that the Docker packages we download are legit (signed by Docker Inc) |
| `lsb-release` | Reports which Ubuntu version you're on | The Docker install script needs to know your Ubuntu version to pick the right package repository |
| `fail2ban` | Brute-force protection | Watches SSH logs for repeated failed login attempts and automatically bans the offending IP via firewall rules. |
| `unattended-upgrades` | Auto-installs security patches | You don't want to manually SSH in every week to run updates. This handles critical security fixes automatically. |

---

### 2. Disable Lid Close Suspend

```bash
HandleLidSwitch=ignore
```

By default, Ubuntu suspends (sleeps) when you close the laptop lid. That's great for a laptop, terrible for a server. This tells the system to do nothing when the lid closes, so you can shut the laptop and tuck it away.

**What's `systemd-logind`?** It's the Linux service that manages user sessions and hardware events like lid switches, power buttons, etc.

---

### 3. Firewall

```bash
ufw default deny incoming    # Block everything coming in
ufw default allow outgoing   # Allow everything going out
ufw allow 22/tcp             # Except SSH (so you can connect)
ufw allow 80/tcp             # Except HTTP (for web traffic)
ufw allow 443/tcp            # Except HTTPS (for secure web traffic)
ufw enable                   # Turn on the firewall
```

Think of it like a bouncer at a door. By default, nobody gets in. Then we put specific ports on the guest list:

- **Port 22 (SSH):** So you can remote in from your Mac
- **Port 80 (HTTP):** So web traffic can reach Traefik
- **Port 443 (HTTPS):** So secure web traffic can reach Traefik

Everything else (databases, random services, etc.) is blocked from the outside. They can still talk to each other internally on the server.

**What's a port?** Think of your server's IP address as a building's street address. Ports are like apartment numbers — port 80 is the "web traffic apartment," port 22 is the "SSH apartment." There are 65,535 ports total. We only open the three we need.

---

### 4. Docker

**What is Docker?**

Docker lets you run applications in "containers" — lightweight, isolated boxes that each have their own filesystem, libraries, and config. Think of it like running mini virtual machines, but way faster and lighter.

Why this matters for the PaaS:
- Each app you deploy becomes a Docker container
- Containers are isolated from each other (one crashing doesn't affect others)
- Easy to start, stop, restart, and delete apps
- The app runs the same way regardless of what's installed on the host

**What's Docker Compose?**

A tool for defining multi-container setups in a YAML file. Instead of running 5 different `docker run` commands, you write a `docker-compose.yml` that describes all your services and run `docker compose up`.

**Why not the snap version?**

Ubuntu offers Docker as a "snap" package (their app store format). We install from Docker's official repository instead because:
- The official version is more up-to-date
- Snap Docker has quirks with file permissions and socket access
- The snap version doesn't include Docker Compose as a plugin

**What's the docker group?**

By default, only `root` can use Docker. Adding your user to the `docker` group lets you run `docker` commands without `sudo` every time.

---

### 5. Create `paas` User

A dedicated user account just for git push deployments:

```bash
git remote add deploy ssh://paas@192.168.1.100/git-repos/my-app.git
git push deploy main
```

**Why a separate user?**

Your `jacob` account has `sudo` (admin) access. The `paas` user can only:
- Receive git pushes
- Interact with Docker (to build and deploy containers)

If someone got access to the `paas` account, they couldn't `sudo rm -rf /` or mess with system files. It's the principle of least privilege — give each account only the permissions it needs.

**`--disabled-password`** means you can't log in as `paas` with a password — SSH key only.

---

### 6. Project Directories

```
/opt/homelab/
├── git-repos/    # Bare git repos (one per app)
└── data/         # SQLite database
```

**What's `/opt`?**

Linux has conventions for where things go:
- `/home` — user files
- `/etc` — system config
- `/var` — variable data (logs, databases)
- `/opt` — optional/self-contained applications

`/opt/homelab` is where our entire PaaS lives.

**What's a bare git repo?**

A normal git repo (like on your Mac) has a `.git` folder and a working directory with all your files. A "bare" repo is just the `.git` contents — no working directory. It's what servers use to receive pushes. GitHub/GitLab store bare repos internally.

When you `git push` to a bare repo, it triggers a "hook" (a script that runs automatically). That's how the PaaS knows to build and deploy your code.

---

### 7. SSH Hardening

The script drops a config file at `/etc/ssh/sshd_config.d/01-hardening.conf`:

```bash
PasswordAuthentication no     # Can't log in with a password
PermitRootLogin no            # Can't log in as root at all
PubkeyAuthentication yes      # Must use SSH keys
MaxAuthTries 3                # Kick after 3 failed attempts
LoginGraceTime 30             # 30 seconds to authenticate or get disconnected
X11Forwarding no              # No GUI forwarding (headless server)
AllowTcpForwarding no         # No SSH tunneling
ClientAliveInterval 300       # Ping idle clients every 5 minutes
ClientAliveCountMax 2         # Disconnect after 2 missed pings (10 min idle timeout)
```

**Why `sshd_config.d/` instead of editing `sshd_config` directly?**

Ubuntu 24.04 uses cloud-init, which drops its own SSH config at `50-cloud-init.conf` with `PasswordAuthentication yes`. In OpenSSH, **first match wins** — files in `sshd_config.d/` are loaded in alphabetical order. By naming ours `01-hardening.conf`, it loads before cloud-init's file and our settings take precedence. The script also removes cloud-init's override to be safe.

**Why disable password auth?**

Passwords can be guessed or brute-forced. SSH keys are basically impossible to crack — they're 2048+ bit encryption keys stored on your Mac. Even if someone knows your password, they can't get in.

**Why disable root login?**

`root` is the all-powerful admin account on Linux. Every server in the world has a `root` user, so it's the first username attackers try. By disabling it, they'd also need to guess your username.

Your `jacob` account can still run admin commands via `sudo` — but attackers would need your SSH key AND your username to even attempt it.

---

### 8. Auto Security Updates

```bash
APT::Periodic::Update-Package-Lists "1";       # Check for updates daily
APT::Periodic::Unattended-Upgrade "1";          # Install security fixes daily
APT::Periodic::AutocleanInterval "7";           # Clean up old packages weekly
```

Linux software gets security patches frequently. This ensures critical fixes are applied automatically without you having to remember. It only installs security updates, not major version changes, so it won't break anything.

The script also configures:
- **Auto-reboot at 3:00 AM** — some security patches (especially kernel updates) don't take effect until a reboot. Without this, the patched kernel would be downloaded but the old vulnerable one keeps running.
- **Unused dependency cleanup** — removes packages that were pulled in as dependencies but are no longer needed after updates.

---

### 9. Kernel Tuning

```bash
vm.swappiness=10         # Only swap under real memory pressure
kernel.panic=10          # Auto-reboot 10 seconds after a kernel panic
kernel.panic_on_oops=1   # Treat kernel oops as panic (reboot instead of hang)
```

**`vm.swappiness=10`** — The default (60) makes Linux start swapping memory to disk even with plenty of free RAM. For a server with 16 GB, `10` means it only swaps when genuinely running low, avoiding unnecessary disk I/O.

**`kernel.panic=10`** — If the kernel crashes, the server would normally freeze forever. On a headless server with no monitor, you'd never know. This makes it automatically reboot after 10 seconds instead.

---

### 10. journald Size Limit

```bash
SystemMaxUse=200M        # Cap total journal size at 200 MB
SystemMaxFileSize=50M    # Cap individual journal files at 50 MB
```

`journald` is the system log daemon — it collects logs from all services, the kernel, SSH, etc. Without a cap, it could eventually consume gigabytes. 200 MB is more than enough for a home server and keeps disk usage predictable.

---

### 11. Docker Network

```bash
docker network create homelab
```

**What's a Docker network?**

By default, Docker containers are isolated — they can't talk to each other. A Docker network is like a virtual LAN that connects containers together.

All our containers join the `homelab` network:
- **Traefik** needs to reach app containers to forward web requests
- **cloudflared** needs to reach Traefik to forward tunnel traffic
- **App containers** might need to reach databases or other services

Containers on the same network can find each other by name. So an app can connect to a database at `homelab-postgres:5432` — Docker's built-in DNS resolves the container name to its IP address.

---

### 12. Static IP

**What's DHCP vs static?**

By default, your router assigns IP addresses dynamically (DHCP). Your laptop might be `192.168.1.235` today and `192.168.1.47` tomorrow. That's fine for phones and laptops — bad for a server you need to reliably SSH into.

A static IP means: "This machine is always `192.168.1.100`, period."

**Why 192.168.1.100?**

Most routers assign DHCP addresses starting around `.100` or `.150` and going up. By picking `.100`, we're at the low end where DHCP usually doesn't assign. Check your router's DHCP range to make sure there's no conflict.

**How does the script detect your network?**

The script figures out whether you're on WiFi or Ethernet:

1. It looks for active WiFi interfaces (names starting with `wl`, like `wlo1` or `wlan0`)
2. If found, it detects your WiFi network name (SSID) using `iwgetid` — a tool that comes with the Linux kernel
3. It reads your WiFi password from the existing netplan config (Ubuntu saves it during install) so you don't have to type it again
4. If no WiFi is found, it looks for a wired ethernet interface — filtering out virtual interfaces like `docker0`, `veth*`, and `br-*` that Docker creates

**What's netplan?**

Ubuntu's network configuration system. You write a YAML file describing how each network interface should behave (DHCP, static IP, WiFi settings, etc.) and apply it with `sudo netplan apply`.

The script writes `/etc/netplan/01-homelab.yaml` with your static IP config. The file is set to permission `600` (only root can read it) since it may contain your WiFi password.

**Why is this last?**

Changing the IP address will disconnect your SSH session (because your Mac is connected to the old IP and you're switching to a new one). So we do it last. The script writes the config but does NOT apply it automatically — you apply it manually with `sudo netplan apply` and then reconnect at the new IP.

---

## After the Script

1. **Apply static IP:** `sudo netplan apply` (will disconnect SSH)
2. **Reconnect:** `ssh jacob@192.168.1.100`
3. **Log out and back in** for the docker group to take effect
4. **Verify Docker:** `docker run hello-world`
5. **Clone the repo:** `cd /opt/homelab && git clone <url> .`

## Useful Commands to Know

| Command | What it does |
|---------|-------------|
| `ssh jacob@192.168.1.100` | Connect to the server from your Mac |
| `sudo` | Run a command as admin (root) |
| `htop` | See CPU, RAM, and running processes |
| `df -h` | See disk usage ("disk free, human-readable") |
| `docker ps` | List running containers |
| `docker logs <name>` | See a container's output |
| `docker stats` | Live CPU/RAM usage per container |
| `sudo ufw status` | See firewall rules |
| `ip addr show` | See network interfaces and IPs |
| `systemctl status <service>` | Check if a service is running |
| `journalctl -u <service> -f` | Stream a service's logs live |
| `reboot` | Restart the server |
