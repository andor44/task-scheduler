-- could imagine other types such as failed, timed-out, etc.
create type task_state as enum ('scheduled', 'executing', 'finished');
create type task_type as enum ('A', 'B', 'C');

create table tasks (
    id         uuid primary key default gen_random_uuid(),
    state      task_state not null default 'scheduled',
    task_type  task_type not null,
    execute_at timestamp with time zone not null

    -- additions i'd definitely want for a prod impl:
    -- 1. mark identity of the executor that claimed a job, so it can resume a failed job
    -- 2. beginning of execution date, so for example deadlines can be respected
    -- 3. payload for tasks, jsonb seems like a good candidate
    -- 4. maybe number of retries?
);