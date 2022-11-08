use std::net::SocketAddr;

use axum::{
    extract::{Extension, Path, Query},
    response::IntoResponse,
    routing, Json, Router,
};
use bb8_postgres::PostgresConnectionManager;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use task_scheduler::{Task, TaskType, TaskState};

type DbPool = bb8::Pool<PostgresConnectionManager<tokio_postgres::NoTls>>;

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

    let app = Router::new()
        .route("/task/", routing::post(create_task))
        .route("/task/:id", routing::delete(delete_task))
        .route("/task/:id", routing::get(show_task))
        .route("/task/", routing::get(list_tasks))
        .layer(Extension(pool));

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Deserialize, Debug)]
struct TaskCreationRequest {
    #[serde(rename = "type")]
    type_: TaskType,
    execution_time: DateTime<Utc>,
}

#[derive(Serialize, Debug)]
struct TaskCreationResponse {
    id: uuid::Uuid,
}

async fn create_task(
    Extension(pool): Extension<DbPool>,
    Json(request): Json<TaskCreationRequest>,
) -> impl IntoResponse {
    let db = pool
        .get()
        .await
        .expect("failed to get a DB connection from the pool");

    let stmt = "insert into tasks (task_type, execute_at) values ($1, $2) returning id";

    let row = db
        .query_one(stmt, &[&request.type_, &request.execution_time])
        .await
        .expect("no result?!");
    let id: uuid::Uuid = row.try_get(0).unwrap();
    axum::Json(TaskCreationResponse { id })
}

async fn delete_task(
    Path(task_id): Path<uuid::Uuid>,
    Extension(pool): Extension<DbPool>,
) -> impl IntoResponse {
    let db = pool
        .get()
        .await
        .expect("failed to get a DB connection from the pool");

    // only allow deleting tasks which have not yet started
    let stmt = "delete from tasks where id = $1 and state = 'scheduled'";

    let rows_affected = db.execute(stmt, &[&task_id]).await.expect("delete failed");
    if rows_affected == 0 {
        axum::http::StatusCode::NOT_FOUND
    } else {
        axum::http::StatusCode::NO_CONTENT
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ShowTaskResponse {
    Datum(Task),
    Error { message: String },
}

async fn show_task(
    Path(task_id): Path<uuid::Uuid>,
    Extension(pool): Extension<DbPool>,
) -> impl IntoResponse {
    let db = pool
        .get()
        .await
        .expect("failed to get a DB connection from the pool");

    let task = task_scheduler::retrieve(&*db, &task_id).await;

    if let Some(task) = task {
        let json = ShowTaskResponse::Datum(task);
        (axum::http::StatusCode::OK, Json(json))
    } else {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(ShowTaskResponse::Error {
                message: "not found".to_string(),
            }),
        )
    }
}

#[derive(Deserialize)]
struct TaskListParams {
    state: Option<TaskState>,
    task_type: Option<TaskType>,
}

async fn list_tasks(
    Extension(pool): Extension<DbPool>,
    params: Query<TaskListParams>,
) -> impl IntoResponse {
    let db = pool
        .get()
        .await
        .expect("failed to get a DB connection from the pool");

    let stmt = "select id, state, task_type, execute_at from tasks where state = ANY($1) and task_type = ANY($2)";

    let default_states = ||vec![TaskState::Scheduled, TaskState::Executing, TaskState::Finished];
    let state_filter = params.state.clone().map_or_else(default_states,|state| vec![state]);

    let default_types = ||vec![TaskType::A, TaskType::B, TaskType::C];
    let type_filter = params.task_type.clone().map_or_else(default_types, |type_| vec![type_]);

    let rows = db.query(stmt, &[&state_filter, &type_filter]).await.expect("failed to query tasks");
    let tasks: Vec<Task> = rows.into_iter().map(Task::from_row).collect();

    Json(tasks)
}
