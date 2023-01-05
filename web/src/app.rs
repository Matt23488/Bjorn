use stylist::yew::styled_component;
use yew::prelude::*;

#[styled_component(App)]
pub fn app() -> Html {
    let counter = use_state(|| 0);
    let onclick = {
        let counter = counter.clone();
        move |_| {
            let value = *counter + 1;
            counter.set(value);
        }
    };

    let container_div = css!(r#"
        display: grid;
        place-items: center;
        width: 100vw;
        height: 100vh;
    "#);

    let counter_div = css!(r#"
        text-align: center;
    "#);

    html! {
        <div class={container_div}>
            <div class={counter_div}>
                <button {onclick}>{"+1"}</button>
                <p>{*counter}</p>
            </div>
        </div>
    }
}