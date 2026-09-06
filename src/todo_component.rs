use dioxus::{logger::tracing, prelude::*};
use uuid::Uuid;

use crate::dao::{TodoItem, create_table_if_exists, get_todos, insert_todo_item, sync_todos};


#[component]
pub fn Controls() -> Element {
    spawn(async move {
        match create_table_if_exists().await {
            Ok(_) => tracing::info!("Tabela criada com sucesso!"),
            Err(e) => tracing::error!("Erro ao criar tabela: {e}"),
        }
    });

    let mut lista = use_signal(|| Vec::<TodoItem>::new());
    let mut texto = use_signal(|| String::new());
    rsx! {
        div { class: "body-center",
            div { class: "custom-input-wrapper",
                h2 { class: "h2-center", "Lista de Tarefas" }
                input {
                    r#type: "text",
                    name: "username",
                    placeholder: "O que precisa ser feito?",
                    oninput: move |evt| texto.set(evt.value()),
                    value: texto,
                    class: "unique-input-field",
                }
                div { class: "button-row-container",
                    button {
                        r#type: "button",
                        class: "btn-action-primary",
                        onclick: move |_| {
                            let item = TodoItem {
                                uuid: Uuid::new_v4().to_string(),
                                description: texto(),
                                done: false,
                            };

                            spawn(async move {
                                texto.clear();
                                insert_todo_item(item).await.unwrap();
                            });
                        },
                        "Salvar"
                    }
                    button {
                        r#type: "button",
                        class: "btn-action-secondary",
                        onclick: move |_| {
                            // Dispara chamada assíncrona para a server function.
                            spawn(async move {
                                match get_todos().await {
                                    Ok(data) => {
                                        lista.set(data);
                                        tracing::info!("Banco de dados sincronizado com sucesso!");
                                    }
                                    Err(err) => tracing::error!("Erro ao sincronizar: {:?}", err),
                                }
                            });
                        },
                        "Sincronizar"
                    }
                }
            }
            TodoList { lista_sinal: lista }
        }
    }
}

#[component]
pub fn TodoList(mut lista_sinal: Signal<Vec<TodoItem>>) -> Element {
    rsx! {
        div { class: "py-8 text-gray-500 max-w-md mx-auto w-full",
            ul { class: "space-y-2 w-full",
                for (index , item) in lista_sinal.iter().enumerate() {
                    li { class: if item.done { "item-list-done" } else { "item-list-not-done" },

                        // flex-1 faz o texto expandir e ocupar todo o espaço,
                        // empurrando o checkbox para a extrema direita.
                        // min-w-0 e truncate evitam que o texto seja cortado.
                        span { class: "flex-1 min-w-0 truncate text-left",
                            "{index} - {item.description}"
                        }

                        input {
                            class: "input-checkbox cursor-pointer flex-shrink-0",
                            r#type: "checkbox",
                            checked: item.done,
                            onchange: move |_| {
                                let mut lista = lista_sinal.write();
                                if let Some(todo) = lista.get_mut(index) {
                                    todo.done = !todo.done;
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}