-- Add migration script here
ALTER TABLE users ADD COLUMN role TEXT NULL;
UPDATE users SET role = 'admin' WHERE username = 'admin';
UPDATE users SET role = 'collaborator' WHERE username != 'admin';
ALTER TABLE users ALTER COLUMN role SET NOT NULL;