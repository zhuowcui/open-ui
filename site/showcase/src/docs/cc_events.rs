use openui::prelude::*;

pub fn example() -> ViewNode {
    let todos = create_signal(vec!["Write the docs".to_string(), "Ship alpha.1".to_string()]);
    let draft = create_signal(String::new());

    let list = view! { <ul style:margin="12px 0 0" style:padding-left="20px"></ul> };
    if let ViewNode::Element(ul) = &list {
        // For keeps one row per key, and adds, removes or moves rows as the list changes.
        let rows = For(
            move || todos.get(),
            |todo| todo.clone(),
            |todo| view! { <li style:padding="2px 0">{todo}</li> },
        );
        mount_view(ul, rows);
    }

    view! {
        <div style:width="360px" style:font-size="14px">
            <div style:display="flex" style:gap="8px">
                <input
                    placeholder="Add a task"
                    style:flex-grow="1"
                    style:padding="6px 10px"
                    style:border="1px solid #d4d4d8"
                    style:border-radius="6px"
                    on:input={move |event: &Event| {
                        let input = event.target().expect("input");
                        draft.set(input.control_value().ok().flatten().unwrap_or_default());
                    }}
                />
                <button
                    style:padding="6px 12px"
                    style:border-radius="6px"
                    style:background-color="#18181b"
                    style:color="#fafafa"
                    on:click={move |_| {
                        let text = draft.get();
                        if !text.is_empty() {
                            todos.update(|list| list.push(text));
                        }
                    }}
                >
                    "Add"
                </button>
            </div>
            <Slot child={list} />
            <p style:margin="8px 0 0" style:color="#71717a">
                {format!("{} tasks", todos.get().len())}
            </p>
        </div>
    }
}

#[component]
fn Slot(child: ViewNode) -> ViewNode {
    child
}
