ALTER TABLE desired_compose_stacks
ADD COLUMN env TEXT[] NOT NULL DEFAULT '{}';
