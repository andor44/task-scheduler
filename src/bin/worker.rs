use task_scheduler::retrieve;

#[tokio::main]
async fn main() {
    let conn_string = "host=localhost user=postgres password=example";
    let manager = bb8_postgres::PostgresConnectionManager::new_from_stringlike(
        conn_string,
        tokio_postgres::NoTls,
    )
    .expect("failed to parse DB connection string");
    let pool = bb8::Pool::builder()
        .build(manager)
        .await
        .expect("failed to create DB connection pool");
    let mut conn = pool
        .get()
        .await
        .expect("failed to get connection from pool");

    loop {
        // begin a transaction
        let tx = conn.transaction().await.expect("failed to start tx");

        // try to fetch a task not claimed by anyone else
        let stmt = "select id from tasks where state = 'scheduled' and execute_at < current_timestamp limit 1 for update skip locked";
        let maybe_task = tx
            .query_opt(stmt, &[])
            .await
            .expect("failed to query for tasks waiting to execute");

        let task_id: uuid::Uuid = if let Some(row) = maybe_task {
            row.get(0)
        } else {
            // if there's no task to work on yield for 5 seconds then start again
            println!("no tasks to work on, waiting...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            continue;
        };

        let stmt = "update tasks set state = 'executing' where id = $1";
        let affected_rows = tx
            .execute(stmt, &[&task_id])
            .await
            .expect("failed to update claimed task as executing");
        if affected_rows != 1 {
            // this should not be possible in theory, as the SELECT ... FOR UPDATE should have locked the row
            // but let's bail out in this case anyway
            println!("claimed task not found for updating as executing, aborting");
            continue;
        }
        // commit the change
        tx.commit().await.expect("failed to commit transaction");

        // fetch task details
        // expect is OK in theory because only scheduled tasks can be deleted, and we've updated ours to be
        // now in executing state
        let task = retrieve(&*conn, &task_id)
            .await
            .expect("task which we claimed not found");

        println!(
            "now executing task id {} of type {:?}",
            task.id, task.task_type
        );

        // pretend to do some work...
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let stmt = "update tasks set state = 'finished' where id = $1";
        conn.execute(stmt, &[&task_id])
            .await
            .expect("failed to update task as completed");
    }
}
