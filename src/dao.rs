use dioxus::prelude::*;

#[cfg(feature = "server")]
use dioxus::logger::tracing;

use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use sqlx::{prelude::FromRow, Pool, Sqlite, SqlitePool};

#[cfg(feature = "server")]
use tokio::sync::OnceCell;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct TodoItem {
    pub uuid: String,
    pub description: String,
    pub done: bool,
}

#[cfg(not(test))]
const SQLITE_CONNECTION_STRING: &str = "sqlite:todos.db?mode=rwc";
#[cfg(test)]
const SQLITE_CONNECTION_STRING: &str = "sqlite::memory:";

#[cfg(feature = "server")]
static POOL: OnceCell<Pool<Sqlite>> = OnceCell::const_new();

#[cfg(feature = "server")]
async fn get_dbpool() -> Result<Pool<Sqlite>, ServerFnError> {
    let pool = POOL
        .get_or_try_init(|| async { SqlitePool::connect(SQLITE_CONNECTION_STRING).await })
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(pool.clone())
}

#[cfg(feature = "server")]
fn map_db_err(e: sqlx::Error) -> ServerFnError {
    tracing::error!("database error: {e}");
    ServerFnError::new(e)
}

#[cfg(feature = "server")]
fn empty_description_err() -> ServerFnError {
    ServerFnError::ServerError {
        message: "description must not be empty".into(),
        code: 400,
        details: None,
    }
}

#[cfg(feature = "server")]
fn not_found_err(uuid: &str) -> ServerFnError {
    ServerFnError::ServerError {
        message: format!("no todo item with uuid {uuid}"),
        code: 404,
        details: None,
    }
}

#[server]
pub async fn insert_todo_item(todo_item: TodoItem) -> Result<(), ServerFnError> {
    if todo_item.description.trim().is_empty() {
        return Err(empty_description_err());
    }

    let pool = get_dbpool().await?;

    sqlx::query(
        "insert into todo_item(uuid, description, done)
        values($1, $2, $3)",
    )
    .bind(&todo_item.uuid)
    .bind(&todo_item.description)
    .bind(todo_item.done)
    .execute(&pool)
    .await
    .map_err(map_db_err)?;

    Ok(())
}

#[server]
pub async fn create_table_if_exists() -> Result<(), ServerFnError> {
    // Conecta ao banco (cria o arquivo 'todos.db' se não existir via 'mode=rwc').
    let pool = get_dbpool().await?;
    sqlx::query(
        "create table if not exists todo_item(
            uuid text primary key,
            description text not null,
            done boolean not null
        )",
    )
    .execute(&pool)
    .await
    .map_err(map_db_err)?;

    Ok(())
}

#[server]
pub async fn update_todo(
    uuid: String,
    description: String,
    done: bool,
) -> Result<(), ServerFnError> {
    if description.trim().is_empty() {
        return Err(empty_description_err());
    }

    let pool = get_dbpool().await?;
    let result = sqlx::query(
        "update todo_item
        set description = $1,
        done = $2
        where uuid = $3
        ",
    )
    .bind(&description)
    .bind(done)
    .bind(&uuid)
    .execute(&pool)
    .await
    .map_err(map_db_err)?;

    if result.rows_affected() == 0 {
        return Err(not_found_err(&uuid));
    }

    Ok(())
}

#[server]
pub async fn get_todos() -> Result<Vec<TodoItem>, ServerFnError> {
    let pool = get_dbpool().await?;

    let buffer = sqlx::query_as::<_, TodoItem>("select * from todo_item")
        .fetch_all(&pool)
        .await
        .map_err(map_db_err)?;

    Ok(buffer)
}

#[server]
pub async fn delete_todo(uuid: String) -> Result<(), ServerFnError> {
    let pool = get_dbpool().await?;
    let result = sqlx::query("delete from todo_item where uuid = $1")
        .bind(&uuid)
        .execute(&pool)
        .await
        .map_err(map_db_err)?;

    if result.rows_affected() == 0 {
        return Err(not_found_err(&uuid));
    }

    Ok(())
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn new_item(description: &str) -> TodoItem {
        TodoItem {
            uuid: Uuid::new_v4().to_string(),
            description: description.to_string(),
            done: false,
        }
    }

    #[tokio::test]
    async fn insert_and_fetch_roundtrip() {
        create_table_if_exists().await.unwrap();
        let item = new_item("write tests");
        insert_todo_item(item.clone()).await.unwrap();

        let todos = get_todos().await.unwrap();
        let found = todos.iter().find(|t| t.uuid == item.uuid).unwrap();
        assert_eq!(found.description, "write tests");
        assert!(!found.done);
    }

    #[tokio::test]
    async fn insert_rejects_empty_description() {
        create_table_if_exists().await.unwrap();
        let item = new_item("   ");

        let err = insert_todo_item(item).await.unwrap_err();
        match err {
            ServerFnError::ServerError { code, .. } => assert_eq!(code, 400),
            other => panic!("expected ServerError with code 400, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn update_toggles_done_and_rejects_unknown_uuid() {
        create_table_if_exists().await.unwrap();
        let item = new_item("toggle me");
        insert_todo_item(item.clone()).await.unwrap();

        update_todo(item.uuid.clone(), item.description.clone(), true)
            .await
            .unwrap();

        let todos = get_todos().await.unwrap();
        let found = todos.iter().find(|t| t.uuid == item.uuid).unwrap();
        assert!(found.done);

        let missing_uuid = Uuid::new_v4().to_string();
        let err = update_todo(missing_uuid, "anything".to_string(), false)
            .await
            .unwrap_err();
        match err {
            ServerFnError::ServerError { code, .. } => assert_eq!(code, 404),
            other => panic!("expected ServerError with code 404, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn delete_removes_item_and_rejects_unknown_uuid() {
        create_table_if_exists().await.unwrap();
        let item = new_item("delete me");
        insert_todo_item(item.clone()).await.unwrap();

        delete_todo(item.uuid.clone()).await.unwrap();

        let todos = get_todos().await.unwrap();
        assert!(!todos.iter().any(|t| t.uuid == item.uuid));

        let missing_uuid = Uuid::new_v4().to_string();
        let err = delete_todo(missing_uuid).await.unwrap_err();
        match err {
            ServerFnError::ServerError { code, .. } => assert_eq!(code, 404),
            other => panic!("expected ServerError with code 404, got {other:?}"),
        }
    }
}
