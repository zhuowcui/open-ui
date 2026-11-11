use openui::prelude::*;

pub fn example() -> ViewNode {
    let steps = ["Install the CLI", "Create an app", "Run it"];
    view! {
        <div style:display="flex" style:gap="40px" style:font-size="15px" style:line-height="1.6">
            // The disc marker hangs outside the list: give the list padding.
            <ul style:margin="0" style:padding-left="20px">
                <li>"Flexbox"</li>
                <li>"Grid"</li>
                <li>"Web fonts"</li>
            </ul>
            // Markers are disc or none, so an ordered list writes its numbers.
            <ol style:margin="0" style:padding="0">
                <li style:list-style="none">{format!("1. {}", steps[0])}</li>
                <li style:list-style="none">{format!("2. {}", steps[1])}</li>
                <li style:list-style="none">{format!("3. {}", steps[2])}</li>
            </ol>
        </div>
    }
}
