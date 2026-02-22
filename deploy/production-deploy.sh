#!/usr/bin/env bash
set -euo pipefail

# Production deploy for darkreach to Hetzner CX22 (178.156.211.107)
# Usage: ./deploy/production-deploy.sh [--skip-searches]
#
# Frontend is deployed to Vercel separately (see frontend/vercel.json).
#
# Performs:
#   1. System hardening (swap, UFW, kernel tuning)
#   2. Nginx reverse proxy install + config (API + WebSocket only)
#   3. Systemd coordinator service
#   4. (Frontend on Vercel — skipped)
#   5. Journald log management
#   6. Launch initial searches
#   7. Verification checks

SERVER="${DEPLOY_USER:-deploy}@178.156.211.107"
SKIP_SEARCHES=false

for arg in "$@"; do
    case "$arg" in
        --skip-searches) SKIP_SEARCHES=true ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "=== Darkreach Production Deploy ==="
echo "Server: $SERVER"
echo ""

# ---------------------------------------------------------------
# Step 1-3, 5: Remote setup (swap, UFW, kernel, nginx, systemd, journald)
# ---------------------------------------------------------------
echo "==> [1/7] System hardening + nginx + systemd + journald"

# Copy config files to server first
scp "$SCRIPT_DIR/nginx-darkreach.conf" "$SERVER:/tmp/nginx-darkreach.conf"
scp "$SCRIPT_DIR/darkreach-coordinator.service" "$SERVER:/tmp/darkreach-coordinator.service"
scp "$SCRIPT_DIR/darkreach-backup.service" "$SERVER:/tmp/darkreach-backup.service"
scp "$SCRIPT_DIR/darkreach-backup.timer" "$SERVER:/tmp/darkreach-backup.timer"
scp "$SCRIPT_DIR/darkreach-certbot-renew.service" "$SERVER:/tmp/darkreach-certbot-renew.service"
scp "$SCRIPT_DIR/darkreach-certbot-renew.timer" "$SERVER:/tmp/darkreach-certbot-renew.timer"
scp "$SCRIPT_DIR/darkreach-alert.service" "$SERVER:/tmp/darkreach-alert.service"
scp "$SCRIPT_DIR/darkreach-alert.timer" "$SERVER:/tmp/darkreach-alert.timer"
scp "$SCRIPT_DIR/darkreach-alert.sh" "$SERVER:/tmp/darkreach-alert.sh"
scp "$SCRIPT_DIR/darkreach-logrotate.conf" "$SERVER:/tmp/darkreach-logrotate.conf"
scp "$SCRIPT_DIR/darkreach-sudoers" "$SERVER:/tmp/darkreach-sudoers"

ssh "$SERVER" bash -s <<'REMOTE_SETUP'
set -euo pipefail

echo "--- [1] Swap file (2GB) ---"
if [ ! -f /swapfile ]; then
    fallocate -l 2G /swapfile
    chmod 600 /swapfile
    mkswap /swapfile
    swapon /swapfile
    echo '/swapfile none swap sw 0 0' >> /etc/fstab
    echo "  Swap created and enabled"
else
    echo "  Swap already exists"
fi
swapon --show

echo "--- [1] UFW firewall ---"
apt-get update -qq
apt-get install -y -qq ufw
ufw default deny incoming
ufw default allow outgoing
ufw allow 22/tcp
ufw allow 80/tcp
ufw allow 443/tcp
echo "y" | ufw enable || true
ufw status

echo "--- [1] Kernel tuning ---"
cat > /etc/sysctl.d/99-darkreach.conf <<'SYSCTL'
net.core.somaxconn = 1024
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 15
vm.swappiness = 10
SYSCTL
sysctl --system >/dev/null 2>&1
echo "  Kernel params applied"

echo "--- [2] Nginx ---"
apt-get install -y -qq nginx
cp /tmp/nginx-darkreach.conf /etc/nginx/sites-available/darkreach
ln -sf /etc/nginx/sites-available/darkreach /etc/nginx/sites-enabled/darkreach
rm -f /etc/nginx/sites-enabled/default
nginx -t
systemctl enable nginx
systemctl restart nginx
echo "  Nginx configured and running"

echo "--- [3] Systemd coordinator ---"
cp /tmp/darkreach-coordinator.service /etc/systemd/system/darkreach-coordinator.service
systemctl daemon-reload
systemctl enable darkreach-coordinator
systemctl restart darkreach-coordinator
echo "  Coordinator service started"

echo "--- [5] Journald log management ---"
mkdir -p /etc/systemd/journald.conf.d
cat > /etc/systemd/journald.conf.d/darkreach.conf <<'JOURNALD'
[Journal]
SystemMaxUse=500M
SystemMaxFileSize=50M
MaxRetentionSec=1week
Compress=yes
JOURNALD
systemctl restart systemd-journald
echo "  Journald configured (500MB cap, 1 week retention)"

echo "--- Sudoers for deploy user ---"
cp /tmp/darkreach-sudoers /etc/sudoers.d/darkreach
chmod 0440 /etc/sudoers.d/darkreach
visudo -c
echo "  Sudoers configured"

echo "--- Backup timer ---"
cp /tmp/darkreach-backup.service /etc/systemd/system/
cp /tmp/darkreach-backup.timer /etc/systemd/system/
mkdir -p /var/backups/darkreach
chown deploy:deploy /var/backups/darkreach
systemctl daemon-reload
systemctl enable --now darkreach-backup.timer
echo "  Backup timer enabled (daily at 03:00 UTC)"

echo "--- Certbot renewal timer ---"
cp /tmp/darkreach-certbot-renew.service /etc/systemd/system/
cp /tmp/darkreach-certbot-renew.timer /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now darkreach-certbot-renew.timer
echo "  Certbot renewal timer enabled (twice daily)"

echo "--- Alert timer ---"
cp /tmp/darkreach-alert.sh /opt/darkreach/deploy/darkreach-alert.sh
chmod +x /opt/darkreach/deploy/darkreach-alert.sh
cp /tmp/darkreach-alert.service /etc/systemd/system/
cp /tmp/darkreach-alert.timer /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now darkreach-alert.timer
echo "  Alert timer enabled (every 5 minutes)"

echo "--- Logrotate ---"
cp /tmp/darkreach-logrotate.conf /etc/logrotate.d/darkreach
echo "  Logrotate configured (nginx logs, daily, 14 days)"

echo "--- Remote setup complete ---"
REMOTE_SETUP

# ---------------------------------------------------------------
# Step 4: Frontend (now on Vercel — nothing to deploy here)
# ---------------------------------------------------------------
echo ""
echo "==> [4/7] Frontend served by Vercel — skipping"

# ---------------------------------------------------------------
# Step 6: Launch initial searches
# ---------------------------------------------------------------
if [ "$SKIP_SEARCHES" = false ]; then
    echo ""
    echo "==> [6/7] Launching initial searches"

    # Wait for coordinator to be ready
    echo "  Waiting for coordinator..."
    for i in $(seq 1 15); do
        if ssh "$SERVER" "curl -sf http://127.0.0.1:7001/api/status >/dev/null 2>&1"; then
            echo "  Coordinator ready"
            break
        fi
        if [ "$i" -eq 15 ]; then
            echo "  WARNING: Coordinator not responding after 15s, skipping searches"
            SKIP_SEARCHES=true
        fi
        sleep 1
    done
fi

if [ "$SKIP_SEARCHES" = false ]; then
    # Palindromic primes: base 10, 11-21 digits
    echo "  Starting palindromic search (base 10, 11-21 digits)..."
    ssh "$SERVER" 'curl -sf -X POST http://127.0.0.1:7001/api/searches \
        -H "Content-Type: application/json" \
        -d '"'"'{"search_type":"palindromic","base":10,"min_digits":11,"max_digits":21}'"'"' || echo "  (may already exist)"'

    # k*b^n +/- 1: k=3, base=2, n=10000-100000
    echo "  Starting kbn search (k=3, 2^n, n=10k-100k)..."
    ssh "$SERVER" 'curl -sf -X POST http://127.0.0.1:7001/api/searches \
        -H "Content-Type: application/json" \
        -d '"'"'{"search_type":"kbn","k":3,"base":2,"min_n":10000,"max_n":100000}'"'"' || echo "  (may already exist)"'

    echo "  Searches launched"
else
    echo ""
    echo "==> [6/7] Skipping searches"
fi

# ---------------------------------------------------------------
# Step 7: Verification
# ---------------------------------------------------------------
echo ""
echo "==> [7/7] Verification"

ssh "$SERVER" bash -s <<'VERIFY'
set -uo pipefail
PASS=0
FAIL=0

check() {
    local label="$1"
    shift
    if eval "$@" >/dev/null 2>&1; then
        echo "  [PASS] $label"
        PASS=$((PASS + 1))
    else
        echo "  [FAIL] $label"
        FAIL=$((FAIL + 1))
    fi
}

check "Swap active"           "swapon --show | grep -q swapfile"
check "UFW enabled"           "ufw status | grep -q 'Status: active'"
check "Nginx running"         "systemctl is-active nginx"
check "Nginx config valid"    "nginx -t 2>&1"
check "Coordinator running"   "systemctl is-active darkreach-coordinator"
check "API responds"          "curl -sf http://127.0.0.1:7001/api/status"
check "Dashboard via nginx"   "curl -sf http://127.0.0.1/api/status"
check "Security headers"      "curl -sI http://127.0.0.1/ | grep -qi 'x-content-type-options'"
check "Backup timer"          "systemctl is-active darkreach-backup.timer"
check "Certbot timer"         "systemctl is-active darkreach-certbot-renew.timer"
check "Alert timer"           "systemctl is-active darkreach-alert.timer"

echo ""
echo "  Results: $PASS passed, $FAIL failed"
VERIFY

echo ""
echo "=== Deploy complete ==="
echo "Dashboard: http://178.156.211.107"
echo ""
echo "Useful commands:"
echo "  ssh $SERVER journalctl -u darkreach-coordinator -f"
echo "  ssh $SERVER systemctl status darkreach-coordinator"
echo "  curl http://178.156.211.107/api/searches"
