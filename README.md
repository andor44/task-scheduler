# Task queue
This is a toy app written in ~3 hours that implements a web API for scheduling
tasks, as well as a really simple executor for them.

The tasks are stored in PostgreSQL, and Postgres' consistency guarantees are
used to enforce semantics, e.g. only tasks which have not been picked up can
be deleted, only one worker can start processing a task, etc.

## Dependencies/storage
Therefore, both the web API and the worker expect there's a PostgreSQL database
available on `localhost:5432`, with user `postgres` and password `example`.
`docker run --rm -it -e POSTGRES_PASSWORD=example -p 5432:5432 postgres` is
sufficient. They also expect that the schema defined in `db.sql` exists.

## Running
* `cargo run --bin webapi`, binds to 0.0.0.0:4000
* `cargo run --bin worker`

## API
There are 4 endpoints on the web API
* `POST /task/` - creates a new task and returns the ID
* `DELETE /task/<id>` - deletes the task matching `id`, given that it hasn't been picked up by a worker yet
* `GET /task/<id>` - retrieve a single task matching `id`
* `GET /task/?state=executing&task_type=B` - list tasks, with optional query parameters `state` and `task_type` filtering for the provided values respectively

## Notes and shortcomings
1. I wrote this with a somewhat-strict 3-hours deadline for myself, therefore
   a lot of low-hanging-fruit was ignored: there's a lot of unwraps/expects
   that could be removed with minimal effort, minimal to no error handling,
   in the worker I just copy-pasted the worker pool creation from the web API
   even though it doesn't need a pool, etc.
1. I believe the idea behind the worker queue based on Postgres row-locking
   is sound, even if the implementation is rather naive by just polling the
   DB for tasks.
1. As it is, if a worker fails while "executing" a task it will be perpetually
   stuck in the `executing` state.
1. The listing of tasks has no pagination as I ran out of time.
1. A complete lack of automated tests. I tested stuff by hand.