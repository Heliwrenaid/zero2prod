-- Add migration script here
CREATE TABLE collabolator_activation_tokens(
    email TEXT PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    created_at timestamptz NOT NULL
);