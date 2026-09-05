use crate::todo_component::TodoItem;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use sqlx::SqlitePool;

// Server functions: O dioxus compila o corpo desta função apenas para o servidor.
#[server]
pub async fn sync_todos(items: Vec<TodoItem>) -> Result<(), ServerFnError> {
    // Conecta ao banco (cria o arquivo 'todos.db' se não existir via 'mode=rwc').
    let pool = SqlitePool::connect("sqlite:todos.db?mode=rwc")
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

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

    // Inicia a transação para o upsert.
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
