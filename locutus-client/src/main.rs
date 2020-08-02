use locutus_client::{
    Model,
    Msg,
};
use yew::App;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::initialize();
    App::<Model>::new().mount_to_body().send_message(Msg::Repaint);
    yew::run_loop();
}
