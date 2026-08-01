ALTER TABLE tb_learning_journeys
ADD COLUMN origin text NOT NULL DEFAULT 'learner',
ADD CONSTRAINT tb_learning_journeys_origin_check CHECK (origin IN ('learner', 'fixture'));

COMMENT ON COLUMN tb_learning_journeys.origin IS
  'Immutable origin for an actual learner journey or a dedicated fixture simulation.';
