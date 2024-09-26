-- Add migration script here
ALTER TABLE issue_delivery_queue ADD COLUMN n_retries SMALLINT DEFAULT 0;
ALTER TABLE issue_delivery_queue ADD COLUMN execute_after TIMESTAMPTZ NULL;