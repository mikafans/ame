-- Collapse role domain from {learner, instructor, admin, agent} to {user, admin, agent}
-- Promotes learner and instructor to the generic 'user' role.

ALTER TABLE tb_users DROP CONSTRAINT tb_users_role_check;

UPDATE tb_users
SET role = 'user'
WHERE role IN ('learner', 'instructor');

ALTER TABLE tb_users ADD CONSTRAINT tb_users_role_check CHECK (role IN ('user', 'admin', 'agent'));
