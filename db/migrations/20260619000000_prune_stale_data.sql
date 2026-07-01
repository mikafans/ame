CREATE OR REPLACE FUNCTION fn_prune_stale_data(dry_run boolean)
RETURNS TABLE(table_name text, would_delete bigint, deleted bigint) AS $$
DECLARE
    v_deleted bigint;
    v_would_delete bigint;
BEGIN
    -- tb_idempotency_keys: created_at < now() - interval '48 hours'
    SELECT COUNT(*) INTO v_would_delete FROM tb_idempotency_keys
    WHERE created_at < now() - interval '48 hours';

    IF dry_run THEN
        v_deleted := 0;
    ELSE
        WITH deleted AS (
            DELETE FROM tb_idempotency_keys
            WHERE ctid IN (
                SELECT ctid FROM tb_idempotency_keys
                WHERE created_at < now() - interval '48 hours'
                LIMIT 10000
            )
            RETURNING 1
        )
        SELECT COUNT(*) INTO v_deleted FROM deleted;

        -- Continue deleting in batches until no more rows
        WHILE v_deleted > 0 LOOP
            WITH deleted AS (
                DELETE FROM tb_idempotency_keys
                WHERE ctid IN (
                    SELECT ctid FROM tb_idempotency_keys
                    WHERE created_at < now() - interval '48 hours'
                    LIMIT 10000
                )
                RETURNING 1
            )
            SELECT COUNT(*) INTO v_deleted FROM deleted;
        END LOOP;
    END IF;
    RETURN QUERY SELECT 'tb_idempotency_keys'::text, v_would_delete, CASE WHEN dry_run THEN 0::bigint ELSE v_would_delete END;

    -- tb_webhook_deliveries: status IN ('delivered','failed') AND created_at < now() - interval '30 days'
    SELECT COUNT(*) INTO v_would_delete FROM tb_webhook_deliveries
    WHERE status IN ('delivered', 'failed') AND created_at < now() - interval '30 days';

    IF dry_run THEN
        v_deleted := 0;
    ELSE
        v_deleted := 0;
        LOOP
            WITH deleted AS (
                DELETE FROM tb_webhook_deliveries
                WHERE ctid IN (
                    SELECT ctid FROM tb_webhook_deliveries
                    WHERE status IN ('delivered', 'failed') AND created_at < now() - interval '30 days'
                    LIMIT 10000
                )
                RETURNING 1
            )
            SELECT COUNT(*) INTO v_deleted FROM deleted;
            EXIT WHEN v_deleted = 0;
        END LOOP;
    END IF;
    RETURN QUERY SELECT 'tb_webhook_deliveries'::text, v_would_delete, CASE WHEN dry_run THEN 0::bigint ELSE v_would_delete END;

    -- tb_messages: status='read' AND created_at < now() - interval '90 days'
    SELECT COUNT(*) INTO v_would_delete FROM tb_messages
    WHERE status = 'read' AND created_at < now() - interval '90 days';

    IF dry_run THEN
        v_deleted := 0;
    ELSE
        v_deleted := 0;
        LOOP
            WITH deleted AS (
                DELETE FROM tb_messages
                WHERE ctid IN (
                    SELECT ctid FROM tb_messages
                    WHERE status = 'read' AND created_at < now() - interval '90 days'
                    LIMIT 10000
                )
                RETURNING 1
            )
            SELECT COUNT(*) INTO v_deleted FROM deleted;
            EXIT WHEN v_deleted = 0;
        END LOOP;
    END IF;
    RETURN QUERY SELECT 'tb_messages'::text, v_would_delete, CASE WHEN dry_run THEN 0::bigint ELSE v_would_delete END;

    -- tb_api_tokens: coalesce(revoked_at, expires_at) < now() - interval '90 days'
    SELECT COUNT(*) INTO v_would_delete FROM tb_api_tokens
    WHERE coalesce(revoked_at, expires_at) < now() - interval '90 days';

    IF dry_run THEN
        v_deleted := 0;
    ELSE
        v_deleted := 0;
        LOOP
            WITH deleted AS (
                DELETE FROM tb_api_tokens
                WHERE ctid IN (
                    SELECT ctid FROM tb_api_tokens
                    WHERE coalesce(revoked_at, expires_at) < now() - interval '90 days'
                    LIMIT 10000
                )
                RETURNING 1
            )
            SELECT COUNT(*) INTO v_deleted FROM deleted;
            EXIT WHEN v_deleted = 0;
        END LOOP;
    END IF;
    RETURN QUERY SELECT 'tb_api_tokens'::text, v_would_delete, CASE WHEN dry_run THEN 0::bigint ELSE v_would_delete END;

    -- tb_assessments: deleted_at IS NOT NULL AND deleted_at < now() - interval '90 days'
    -- Real cascading delete via FK constraints
    SELECT COUNT(*) INTO v_would_delete FROM tb_assessments
    WHERE deleted_at IS NOT NULL AND deleted_at < now() - interval '90 days';

    IF dry_run THEN
        v_deleted := 0;
    ELSE
        v_deleted := 0;
        LOOP
            WITH deleted AS (
                DELETE FROM tb_assessments
                WHERE ctid IN (
                    SELECT ctid FROM tb_assessments
                    WHERE deleted_at IS NOT NULL AND deleted_at < now() - interval '90 days'
                    LIMIT 10000
                )
                RETURNING 1
            )
            SELECT COUNT(*) INTO v_deleted FROM deleted;
            EXIT WHEN v_deleted = 0;
        END LOOP;
    END IF;
    RETURN QUERY SELECT 'tb_assessments'::text, v_would_delete, CASE WHEN dry_run THEN 0::bigint ELSE v_would_delete END;

    -- tb_audit_log: created_at < now() - interval '365 days'
    SELECT COUNT(*) INTO v_would_delete FROM tb_audit_log
    WHERE created_at < now() - interval '365 days';

    IF dry_run THEN
        v_deleted := 0;
    ELSE
        v_deleted := 0;
        LOOP
            WITH deleted AS (
                DELETE FROM tb_audit_log
                WHERE ctid IN (
                    SELECT ctid FROM tb_audit_log
                    WHERE created_at < now() - interval '365 days'
                    LIMIT 10000
                )
                RETURNING 1
            )
            SELECT COUNT(*) INTO v_deleted FROM deleted;
            EXIT WHEN v_deleted = 0;
        END LOOP;
    END IF;
    RETURN QUERY SELECT 'tb_audit_log'::text, v_would_delete, CASE WHEN dry_run THEN 0::bigint ELSE v_would_delete END;

    -- tb_activity_log: ts < now() - interval '90 days' AND tool_name <> 'attempt.grade'
    SELECT COUNT(*) INTO v_would_delete FROM tb_activity_log
    WHERE ts < now() - interval '90 days' AND tool_name <> 'attempt.grade';

    IF dry_run THEN
        v_deleted := 0;
    ELSE
        v_deleted := 0;
        LOOP
            WITH deleted AS (
                DELETE FROM tb_activity_log
                WHERE ctid IN (
                    SELECT ctid FROM tb_activity_log
                    WHERE ts < now() - interval '90 days' AND tool_name <> 'attempt.grade'
                    LIMIT 10000
                )
                RETURNING 1
            )
            SELECT COUNT(*) INTO v_deleted FROM deleted;
            EXIT WHEN v_deleted = 0;
        END LOOP;
    END IF;
    RETURN QUERY SELECT 'tb_activity_log'::text, v_would_delete, CASE WHEN dry_run THEN 0::bigint ELSE v_would_delete END;
END;
$$ LANGUAGE plpgsql;
