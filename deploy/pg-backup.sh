#!/usr/bin/env bash
# Daily PostgreSQL backup for darkreach
# Intended to run via cron: 0 3 * * * /opt/darkreach/deploy/pg-backup.sh
set -euo pipefail

BACKUP_DIR="${BACKUP_DIR:-/var/backups/darkreach}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/darkreach_${TIMESTAMP}.sql.gz"

mkdir -p "$BACKUP_DIR"

echo "[$(date)] Starting backup to ${BACKUP_FILE}"
pg_dump -U darkreach -d darkreach \
    --format=custom \
    --compress=6 \
    --verbose \
    --file="${BACKUP_FILE}" 2>&1

FILESIZE=$(du -h "$BACKUP_FILE" | cut -f1)
echo "[$(date)] Database backup complete: ${BACKUP_FILE} (${FILESIZE})"

# Back up checkpoint file if it exists
CHECKPOINT_FILE="${CHECKPOINT_FILE:-/opt/darkreach/darkreach.checkpoint}"
if [ -f "$CHECKPOINT_FILE" ]; then
    CHECKPOINT_BACKUP="${BACKUP_DIR}/checkpoint_${TIMESTAMP}.json"
    cp "$CHECKPOINT_FILE" "$CHECKPOINT_BACKUP"
    echo "[$(date)] Checkpoint backed up: ${CHECKPOINT_BACKUP}"
fi

# Verify backup integrity
pg_restore --list "$BACKUP_FILE" > /dev/null 2>&1
echo "[$(date)] Backup verified"

# Prune old backups
DELETED=$(find "$BACKUP_DIR" -name "darkreach_*.sql.gz" -mtime +"$RETENTION_DAYS" -delete -print | wc -l)
if [ "$DELETED" -gt 0 ]; then
    echo "[$(date)] Pruned ${DELETED} backups older than ${RETENTION_DAYS} days"
fi

echo "[$(date)] Backup complete"
