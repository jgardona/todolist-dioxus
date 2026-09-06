use dioxus::prelude::*;

use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use sqlx::{prelude::FromRow, Pool, Sqlite, SqlitePool};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct TodoItem {
    pub uuid: String,
    pub description: String,
    pub done: bool,
}

const SQLITE_CONNECTION_STRING: &str = "sqlite:todos.db?mode=rwc";

#[cfg(feature = "server")]
async fn get_dbpool() -> Result<Pool<Sqlite>, ServerFnError> {
    let pool = SqlitePool::connect(SQLITE_CONNECTION_STRING)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(pool)
}

#[server]
pub async fn insert_todo_item(todo_item: TodoItem) -> Result<(), ServerFnError> {
    let pool = get_dbpool().await?;
    let mut tx = pool.begin().await.map_err(|e| ServerFnError::new(e))?;

    sqlx::query(
        "insert into todo_item(uuid, description, done)
        values($1, $2, $3)",
    )
    .bind(&todo_item.uuid)
    .bind(&todo_item.description)
    .bind(&todo_item.done)
    .execute(&mut *tx)
    .await
    .map_err(|e| ServerFnError::new(e))?;
    tx.commit().await.map_err(|e| ServerFnError::new(e))?;

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
    .map_err(|e| ServerFnError::new(e))?;

    Ok(())
}

#[server]
pub async fn get_todos() -> Result<Vec<TodoItem>, ServerFnError> {
    let pool = get_dbpool().await?;

    let buffer = sqlx::query_as::<_, TodoItem>("select * from todo_item")
        .fetch_all(&pool)
        .await
        .map_err(|e| ServerFnError::new(e))?;

    Ok(buffer)
}

#[server]
pub async fn sync_todos(items: Vec<TodoItem>) -> Result<(), ServerFnError> {
    // Inicia a transação para o upsert.
    let pool = get_dbpool().await?;
    let mut tx = pool.begin().await.map_err(|e| ServerFnError::new(e))?;

    for item in items {
        sqlx::query(
            "INSERT OR REPLACE INTO todo_item (uuid, description, done) VALUES ($1, $2, $3)",
        )
        .bind(&item.uuid)
        .bind(&item.description)
        .bind(&item.done)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e))?;
    }

    // Efetiva a transação no SQLite.
    tx.commit().await.map_err(|e| ServerFnError::new(e))?;
    Ok(())
}
