# Darkreach Recovery Procedures

Runbook for recovering from common failure scenarios.

## Coordinator Crash

**Symptoms**: `systemctl status darkreach-coordinator` shows `failed` or `inactive`.

```bash
# Check status and recent logs
ssh deploy@178.156.211.107
sudo systemctl status darkreach-coordinator
journalctl -u darkreach-coordinator -n 100 --no-pager

# Restart the coordinator
sudo systemctl restart darkreach-coordinator

# Verify it's healthy
curl -sf http://127.0.0.1:7001/healthz
curl -sf http://127.0.0.1:7001/healthz/deep | python3 -m json.tool
```

**If restart loops** (check `StartLimitBurst`):
```bash
# Reset the failure counter
sudo systemctl reset-failed darkreach-coordinator
sudo systemctl start darkreach-coordinator
```

**If the binary is corrupted**:
```bash
# Rebuild from source
cd /opt/darkreach
git pull origin master
RUSTFLAGS="-C target-cpu=native" cargo build --release
sudo systemctl stop darkreach-coordinator
sudo cp target/release/darkreach /usr/local/bin/darkreach
sudo systemctl start darkreach-coordinator
```

## Database Crash

**Symptoms**: `/healthz/deep` shows `database: unhealthy`, or Supabase dashboard shows errors.

```bash
# Check Supabase project status via dashboard:
# https://supabase.com/dashboard

# If using self-hosted PostgreSQL:
sudo systemctl status postgresql
journalctl -u postgresql -n 50 --no-pager

# Restart PostgreSQL
sudo systemctl restart postgresql
```

**Restore from backup**:
```bash
# List available backups
ls -la /var/backups/darkreach/

# Restore the latest backup
LATEST=$(ls -t /var/backups/darkreach/darkreach_*.sql.gz | head -1)
pg_restore -U darkreach -d darkreach --clean --if-exists "$LATEST"

# Verify
psql -U darkreach -d darkreach -c "SELECT count(*) FROM primes;"
```

## Worker Crash

**Symptoms**: Worker not visible in dashboard fleet view, `systemctl status darkreach-worker@N` shows failed.

```bash
# Check worker status
ssh deploy@WORKER_HOST
sudo systemctl status darkreach-worker@1

# Check logs for the crash reason
journalctl -u darkreach-worker@1 -n 100 --no-pager

# Restart the worker
sudo systemctl restart darkreach-worker@1

# Verify it reconnected (from coordinator)
curl -sf http://127.0.0.1:7001/api/workers | python3 -m json.tool
```

**If checkpoint is corrupted**:
```bash
# Remove the corrupted checkpoint (work will restart from the beginning of the block)
rm /opt/darkreach/darkreach-1.checkpoint
sudo systemctl restart darkreach-worker@1
```

## Full Server Recovery

For complete server loss (e.g., Hetzner instance replacement):

1. **Provision new server** (CX22 or equivalent)
2. **Run production deploy**:
   ```bash
   ./deploy/production-deploy.sh
   ```
3. **Restore database** from latest backup (see Database Crash above)
4. **Restore checkpoint** if available:
   ```bash
   LATEST_CP=$(ls -t /var/backups/darkreach/checkpoint_*.json | head -1)
   cp "$LATEST_CP" /opt/darkreach/darkreach.checkpoint
   ```
5. **Verify all services**:
   ```bash
   sudo systemctl status darkreach-coordinator
   sudo systemctl status darkreach-backup.timer
   sudo systemctl status darkreach-certbot-renew.timer
   sudo systemctl status darkreach-alert.timer
   curl -sf http://127.0.0.1:7001/healthz/deep | python3 -m json.tool
   ```

## Disk Full

**Symptoms**: `/healthz/deep` shows `disk: unhealthy`, services failing to write.

```bash
# Check disk usage
df -h /

# Common culprits:
# 1. Old backups
find /var/backups/darkreach -name "*.sql.gz" -mtime +30 -delete

# 2. Journal logs
sudo journalctl --vacuum-size=200M

# 3. Old Rust build artifacts
rm -rf /opt/darkreach/target/debug
cargo clean --manifest-path /opt/darkreach/Cargo.toml

# 4. Nginx logs (if logrotate isn't running)
sudo truncate -s 0 /var/log/nginx/access.log
sudo truncate -s 0 /var/log/nginx/error.log
```

## TLS Certificate Expiry

**Symptoms**: Alert script reports certificate expiring, browsers show TLS warnings.

```bash
# Check current certificate
echo | openssl s_client -servername darkreach.ai -connect darkreach.ai:443 2>/dev/null \
    | openssl x509 -noout -dates

# Force renewal
sudo certbot renew --force-renewal
sudo systemctl reload nginx

# Verify
curl -sI https://darkreach.ai | head -5
```

## Alert Reference

| Alert | Trigger | Recovery |
|-------|---------|----------|
| `coordinator_down` | `/healthz` unreachable | Restart coordinator service |
| `deep_unhealthy` | `/healthz/deep` returns unhealthy | Check individual component statuses |
| `disk_low` | < 2GB free on / | Free disk space (see Disk Full) |
| `backup_stale` | No backup in last 26h | Check backup timer, run manually |
| `backup_missing` | No backups in backup dir | Run `pg-backup.sh` manually |
| `cert_expiring` | TLS cert expires in < 14 days | Run `certbot renew` |

## Useful Commands

```bash
# View coordinator logs (live)
journalctl -u darkreach-coordinator -f

# View all darkreach service logs
journalctl -u 'darkreach-*' --since "1 hour ago"

# Check all timers
systemctl list-timers 'darkreach-*'

# Manual backup
/opt/darkreach/deploy/pg-backup.sh

# Manual alert check
/opt/darkreach/deploy/darkreach-alert.sh

# Deep health check
curl -sf http://127.0.0.1:7001/healthz/deep | python3 -m json.tool
```
