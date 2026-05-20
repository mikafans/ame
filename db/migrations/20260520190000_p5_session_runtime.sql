alter table questions
  drop constraint questions_kind_check,
  add constraint questions_kind_check
    check (kind in ('mc', 'tf', 'short', 'essay', 'code'));

alter table questions
  add column points integer not null default 1,
  add constraint questions_points_nonnegative check (points >= 0);

alter table sessions
  add column quiz_id uuid;

alter table sessions
  drop constraint sessions_exam_link,
  add constraint sessions_kind_link
    check (
      (kind = 'exam' and exam_id is not null and quiz_id is null)
      or (kind = 'quiz' and quiz_id is not null and exam_id is null)
      or (kind = 'practice' and quiz_id is null and exam_id is null)
    );

create unique index attempts_session_question_unique
  on attempts(session_id, question_id)
  where session_id is not null;
