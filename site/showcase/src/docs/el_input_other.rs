use openui::prelude::*;

// These types are drawn, but open no picker or file dialog yet.
pub fn example() -> ViewNode {
    view! {
        <div style:display="flex" style:gap="12px" style:align-items="center" style:font-size="14px">
            <input type="color" style:width="44px" style:height="28px" />
            <input type="date" style:width="150px" style:height="28px" style:font-size="14px" />
            <input type="file" style:width="230px" style:height="28px" style:font-size="14px" />
        </div>
    }
}
