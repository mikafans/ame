-- Plan 10 (inline P4): quiz authoring schema additions
-- objectives on quizzes, quiz_questions join table

alter table quizzes
  add column objectives text[] not null default '{}';

create table quiz_questions (
  quiz_id       uuid not null references quizzes(id) on delete cascade,
  question_id   uuid not null references questions(id) on delete cascade,
  order_index   int  not null,
  points_override int,
  primary key (quiz_id, question_id),
  unique (quiz_id, order_index)
);
