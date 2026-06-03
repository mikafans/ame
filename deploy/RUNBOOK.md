# AME Cluster Operations & Administration Runbook

This document details common administrative and operational tasks for the AME platform in production.

## Table of Contents
1. [Database Backups & Restore](#1-database-backups--restore)
2. [Secret Rotation](#2-secret-rotation)
3. [Admin Promotion](#3-admin-promotion)
4. [Deployment Rollbacks](#4-deployment-rollbacks)
5. [Upgrade to S3 / Object Storage](#5-upgrade-to-s3--object-storage)

---

## 1. Database Backups & Restore

### Backups
PostgreSQL database backups are taken automatically on a daily schedule at 2:00 AM using a Kubernetes `CronJob` (`ame-backup`).
- **Storage Location**: Backups are written as PostgreSQL custom format (`.dump`) files to a Persistent Volume Claim (`ame-backup`).
- **Retention**: The backup job retains only the last 7 daily backup files.

### Backup File Inspection (Production)
To check the existing backups inside the cluster:
```bash
kubectl exec -it deployment/ame-api -c backup -- ls -lh /backups
```
*(Or target the backup pod directly)*

### Database Restore Procedure

> [!WARNING]
> A restore operation will overwrite the current database state. In a disaster recovery situation, always attempt to take a manual snapshot/dump of the current state before restoring an older backup.

#### Step 1: Perform a Dry-run Restore (Recommended)
Before restoring to the active production database, perform a test restore into a temporary scratch database (e.g. `ame_scratch`) to verify dump integrity:
1. Create a scratch database:
   ```bash
   kubectl exec -it statefulset/ame-postgres -- createdb -U postgres ame_scratch
   ```
2. Restore the backup file into the scratch database:
   ```bash
   kubectl exec -i statefulset/ame-postgres -- pg_restore -U postgres -d ame_scratch --clean --if-exists --no-owner --no-privileges < /path/to/backup.dump
   ```
3. Run a quick count check to verify data integrity:
   ```bash
   kubectl exec -it statefulset/ame-postgres -- psql -U postgres -d ame_scratch -c "SELECT COUNT(*) FROM tb_users;"
   ```
4. Drop the scratch database:
   ```bash
   kubectl exec -it statefulset/ame-postgres -- dropdb -U postgres ame_scratch
   ```

#### Step 2: Restore to the Production Database (`ame`)
If the dry-run passes:
1. Scale down the API application to stop active connections to the database:
   ```bash
   kubectl scale deployment/ame-api --replicas=0
   ```
2. Execute the restore directly:
   ```bash
   kubectl exec -i statefulset/ame-postgres -- pg_restore -U postgres -d ame --clean --if-exists --no-owner --no-privileges < /path/to/backup.dump
   ```
3. Scale the API application back up:
   ```bash
   kubectl scale deployment/ame-api --replicas=1
   ```

---

## 2. Secret Rotation

In the event of a credential leak, secrets can be rotated by updating the Kubernetes secret definition.

### Rotating the PostgreSQL Password

1. Generate a new password.
2. Update the Kubernetes Secret (`ame-secrets`):
   ```bash
   kubectl create secret generic ame-secrets \
     --from-literal=POSTGRES_PASSWORD='<NEW_PASSWORD>' \
     --dry-run=client -o yaml | kubectl apply -f -
   ```
3. Perform a rolling restart of the database StatefulSet:
   ```bash
   kubectl rollout restart statefulset/ame-postgres
   ```
4. Perform a rolling restart of the API deployment so it picks up the new credentials:
   ```bash
   kubectl rollout restart deployment/ame-api
   ```

### Rotating the Agent Access Code / JWT Secrets
JWT tokens or agent keys are read from environment variables or custom configuration maps/secrets.
To rotate the agent access code (`AME_AGENT_ACCESS_CODE`):
1. Update the environment configuration file or secret.
2. Restart the API deployment:
   ```bash
   kubectl rollout restart deployment/ame-api
   ```

---

## 3. Admin Promotion

Since public registration restricts role grants to `user` for safety, new administrator accounts must be promoted out-of-band directly in the database.

### Option A: Local / Helper Script (Makefile)
If running locally, you can promote an email via:
```bash
make db-admin ADMIN_EMAIL="admin@example.com"
```

### Option B: Production SQL promotion
To promote an existing registered user to `admin` in Kubernetes:
```bash
kubectl exec -it statefulset/ame-postgres -- psql -U postgres -d ame -c \
  "UPDATE tb_users SET role = 'admin' WHERE email = 'admin@example.com';"
```

---

## 4. Deployment Rollbacks

If a new deploy introduces critical issues or fails the `/readyz` probe:

### Step 1: Identify the Issue
Check the rollout history and status:
```bash
kubectl rollout history deployment/ame-api
kubectl rollout status deployment/ame-api
```

### Step 2: Rollback to a Stable Revision
Rollback the api deployment:
```bash
kubectl rollout undo deployment/ame-api
```
Rollback the frontend deployment:
```bash
kubectl rollout undo deployment/ame-web
```

---

## 5. Upgrade to S3 / Object Storage

Storing backups on a PVC is simple but carries risk if the underlying storage node or zone fails. Upgrading the backups to upload directly to S3 or compatible object storage (like MinIO) is highly recommended.

### Migration Steps:
1. **Create an S3 Bucket** (e.g. `ame-db-backups`).
2. **Add Credentials** to `ame-secrets`:
   - `AWS_ACCESS_KEY_ID`
   - `AWS_SECRET_ACCESS_KEY`
   - `AWS_DEFAULT_REGION`
3. **Modify the Backup Container**:
   Change the image to an AWS CLI/postgres client image (e.g., `amazon/aws-cli` or similar, or install `aws-cli` via alpine package manager).
   Update `command` to upload the dump file directly to S3:
   ```bash
   aws s3 cp "${BACKUP_DIR}/${FILENAME}" "s3://ame-db-backups/${FILENAME}"
   ```
4. **Clean up Local Storage**:
   You can either keep the local backup directory as a secondary cache or remove the local volume mounting entirely.
