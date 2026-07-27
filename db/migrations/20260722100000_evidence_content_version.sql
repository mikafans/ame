ALTER TABLE tb_mastery_evidence
ADD COLUMN content_version integer NOT NULL DEFAULT 1;

ALTER TABLE tb_mastery_evidence
ADD CONSTRAINT tb_mastery_evidence_content_version_check
CHECK (content_version > 0);
