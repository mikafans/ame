-- Plan 6: exam sections table + missing exam columns

alter table exams
  add column if not exists method          text not null default 'manual',
  add column if not exists duration_min    integer,
  add column if not exists total_points    integer not null default 0,
  add column if not exists passing_points  integer,
  add column if not exists objectives      text[] not null default '{}',
  add column if not exists composition_trace jsonb,
  add constraint exams_method_check check (method in ('manual', 'agent')),
  add constraint exams_status_check  check (status in ('draft', 'published', 'archived'));

-- exam_sections: one row per section; resolved at composition time.
-- Static section:  question_ids non-null, mix null.
-- Dynamic section: question_ids null, mix + items_count non-null.
create table exam_sections (
  id            uuid primary key default uuid_generate_v7(),
  exam_id       uuid not null references exams(id) on delete cascade,
  title         text not null,
  order_index   integer not null,
  weight        double precision not null default 1.0,
  question_ids  uuid[],                     -- static: explicit list
  mix           jsonb,                       -- dynamic: {tags, types, difficulty_min?, difficulty_max?}
  items_count   integer not null,            -- number of questions planned for this section
  created_at    timestamptz not null default now(),
  constraint exam_sections_kind_check
    check ((question_ids is not null and mix is null)
        or (question_ids is null and mix is not null))
);
create index exam_sections_exam on exam_sections(exam_id, order_index);
