use dioxus::{logger::tracing, prelude::*};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::dao::sync_todos;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodoItem {
    pub uuid: String,
    pub description: String,
    pub done: bool,
}

#[component]
pub fn Controls() -> Element {
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
                            lista.push(item);
                        },
                        "Salvar"
                    }
                    button {
                        r#type: "button",
                        class: "btn-action-secondary",
                        onclick: move |_| {
                            // Dispara chamada assíncrona para a server function.
                            spawn(async move {
                                match sync_todos(lista()).await {
                                    Ok(_) => tracing::info!("Banco de dados sincronizado com sucesso!"),
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
        div { class: "text-center py-8 text-gray-500",
            ul {
                for (index , item) in lista_sinal.iter().enumerate() {
                    li { class: if item.done { "item-list-done" } else { "item-list-not-done" },
                        "{item.uuid} - {item.description}"
                        input {
                            class: "input-checkbox",
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
