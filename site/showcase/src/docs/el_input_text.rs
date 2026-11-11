use openui::prelude::*;

pub fn example() -> ViewNode {
    let name = create_signal(String::from("Ada"));
    view! {
        <div style:width="300px" style:font-size="14px">
            <label style:display="block" style:font-weight="500" style:margin-bottom="6px">
                "Name"
            </label>
            <input
                value="Ada"
                placeholder="Your name"
                style:box-sizing="border-box"
                style:width="100%"
                style:padding="8px 10px"
                style:border="1px solid #d4d4d8"
                style:border-radius="8px"
                style:font-size="14px"
                on:input={move |event: &Event| {
                    let input = event.target().expect("input target");
                    if let Ok(Some(value)) = input.control_value() {
                        name.set(value);
                    }
                }}
            />
            <p style:margin="8px 0 0" style:color="#71717a">
                {format!("Hello, {}!", name.get())}
            </p>
            <label style:display="block" style:font-weight="500" style:margin="16px 0 6px">
                "Password"
            </label>
            <input
                type="password"
                value="hunter2"
                style:box-sizing="border-box"
                style:width="100%"
                style:padding="8px 10px"
                style:border="1px solid #d4d4d8"
                style:border-radius="8px"
                style:font-size="14px"
            />
        </div>
    }
}
