use postgres_types::{FromSql, ToSql};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, ToSql, FromSql, Clone)]
#[postgres(name = "task_type")]
pub enum TaskType {
    A,
    B,
    C,
}

#[derive(Deserialize, Serialize, Debug, ToSql, FromSql, Clone)]
#[postgres(name = "task_state")]
pub enum TaskState {
    #[postgres(name = "scheduled")]
    Scheduled,
    #[postgres(name = "executing")]
    Executing,
    #[postgres(name = "finished")]
    Finished,
}

#[derive(Serialize)]
pub struct Task {
    pub id: uuid::Uuid,
    pub state: TaskState,
    pub task_type: TaskType,
    pub execute_at: chrono::DateTime<chrono::Utc>,
}

impl Task {
    pub fn from_row(row: tokio_postgres::Row) -> Task {
        Task {
            id: row.get(0),
            state: row.get(1),
            task_type: row.get(2),
            execute_at: row.get(3),
        }
    }
}

pub async fn retrieve<T: tokio_postgres::GenericClient>(db: &T, id: &uuid::Uuid) -> Option<Task> {
    let stmt = "select id, state, task_type, execute_at from tasks where id = $1";
    let row = db.query_opt(stmt, &[id]).await.expect("query failed");
    row.map(Task::from_row)
}
