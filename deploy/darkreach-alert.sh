#!/usr/bin/env bash
# Darkreach health alerting script.
# Checks coordinator health, disk space, backup freshness, and TLS cert expiry.
# Sends alerts to a webhook URL (Discord/Slack) when issues are detected.
#
# Runs every 5 minutes via darkreach-alert.timer.
# Deduplicates alerts using a state file to avoid spam.
set -euo pipefail

STATE_FILE="${STATE_FILE:-/var/lib/darkreach/alert-state}"
WEBHOOK_URL="${DARKREACH_ALERT_WEBHOOK:-}"
COORDINATOR_URL="${COORDINATOR_URL:-http://127.0.0.1:7001}"
DOMAIN="${DOMAIN:-darkreach.ai}"
BACKUP_DIR="${BACKUP_DIR:-/var/backups/darkreach}"
BACKUP_MAX_AGE_HOURS="${BACKUP_MAX_AGE_HOURS:-26}"

mkdir -p "$(dirname "$STATE_FILE")"
touch "$STATE_FILE"

ALERTS=""

send_alert() {
    local message="$1"
    if [ -z "$WEBHOOK_URL" ]; then
        echo "[ALERT] $message (no webhook configured)"
        return
    fi
    # Discord/Slack compatible webhook payload
    curl -sf -X POST "$WEBHOOK_URL" \
        -H "Content-Type: application/json" \
        -d "{\"content\":\"⚠️ **darkreach alert**: ${message}\"}" \
        >/dev/null 2>&1 || echo "[WARN] Failed to send webhook"
}

# Check if an alert was already sent (dedup by key)
already_alerted() {
    local key="$1"
    grep -q "^${key}$" "$STATE_FILE" 2>/dev/null
}

# Mark an alert as sent
mark_alerted() {
    local key="$1"
    if ! already_alerted "$key"; then
        echo "$key" >> "$STATE_FILE"
    fi
}

# Clear an alert (issue resolved)
clear_alert() {
    local key="$1"
    if [ -f "$STATE_FILE" ]; then
        grep -v "^${key}$" "$STATE_FILE" > "${STATE_FILE}.tmp" 2>/dev/null || true
        mv "${STATE_FILE}.tmp" "$STATE_FILE"
    fi
}

# 1. Coordinator health check
if ! curl -sf "${COORDINATOR_URL}/healthz" >/dev/null 2>&1; then
    if ! already_alerted "coordinator_down"; then
        send_alert "Coordinator is DOWN (${COORDINATOR_URL}/healthz unreachable)"
        mark_alerted "coordinator_down"
    fi
else
    clear_alert "coordinator_down"
fi

# 2. Deep health check
DEEP_STATUS=$(curl -sf "${COORDINATOR_URL}/healthz/deep" 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('status','unknown'))" 2>/dev/null || echo "unreachable")
if [ "$DEEP_STATUS" = "unhealthy" ]; then
    if ! already_alerted "deep_unhealthy"; then
        send_alert "Deep health check: UNHEALTHY — check ${COORDINATOR_URL}/healthz/deep"
        mark_alerted "deep_unhealthy"
    fi
elif [ "$DEEP_STATUS" = "ok" ] || [ "$DEEP_STATUS" = "degraded" ]; then
    clear_alert "deep_unhealthy"
fi

# 3. Disk space check (< 2GB free)
DISK_FREE_KB=$(df / --output=avail 2>/dev/null | tail -1 | tr -d ' ' || echo "0")
DISK_FREE_GB=$((DISK_FREE_KB / 1048576))
if [ "$DISK_FREE_GB" -lt 2 ]; then
    if ! already_alerted "disk_low"; then
        send_alert "Disk space critically low: ${DISK_FREE_GB}GB free"
        mark_alerted "disk_low"
    fi
else
    clear_alert "disk_low"
fi

# 4. Backup freshness check
if [ -d "$BACKUP_DIR" ]; then
    LATEST_BACKUP=$(find "$BACKUP_DIR" -name "darkreach_*.sql.gz" -type f -printf '%T@\n' 2>/dev/null | sort -rn | head -1 || echo "0")
    if [ -n "$LATEST_BACKUP" ] && [ "$LATEST_BACKUP" != "0" ]; then
        NOW=$(date +%s)
        BACKUP_AGE=$(( (NOW - ${LATEST_BACKUP%.*}) / 3600 ))
        if [ "$BACKUP_AGE" -gt "$BACKUP_MAX_AGE_HOURS" ]; then
            if ! already_alerted "backup_stale"; then
                send_alert "Last backup is ${BACKUP_AGE}h old (threshold: ${BACKUP_MAX_AGE_HOURS}h)"
                mark_alerted "backup_stale"
            fi
        else
            clear_alert "backup_stale"
        fi
    else
        if ! already_alerted "backup_missing"; then
            send_alert "No backups found in ${BACKUP_DIR}"
            mark_alerted "backup_missing"
        fi
    fi
fi

# 5. TLS certificate expiry check (< 14 days)
if command -v openssl &>/dev/null; then
    CERT_EXPIRY=$(echo | openssl s_client -servername "$DOMAIN" -connect "${DOMAIN}:443" 2>/dev/null | openssl x509 -noout -enddate 2>/dev/null | cut -d= -f2 || echo "")
    if [ -n "$CERT_EXPIRY" ]; then
        EXPIRY_EPOCH=$(date -d "$CERT_EXPIRY" +%s 2>/dev/null || date -j -f "%b %d %T %Y %Z" "$CERT_EXPIRY" +%s 2>/dev/null || echo "0")
        NOW=$(date +%s)
        DAYS_LEFT=$(( (EXPIRY_EPOCH - NOW) / 86400 ))
        if [ "$DAYS_LEFT" -lt 14 ] && [ "$DAYS_LEFT" -ge 0 ]; then
            if ! already_alerted "cert_expiring"; then
                send_alert "TLS certificate for ${DOMAIN} expires in ${DAYS_LEFT} days"
                mark_alerted "cert_expiring"
            fi
        else
            clear_alert "cert_expiring"
        fi
    fi
fi

echo "[$(date)] Alert check complete"
