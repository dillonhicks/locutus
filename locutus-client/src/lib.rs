#![recursion_limit = "1024"]

use crate::deps::{
    anyhow::Error,
    serde::{Deserialize, Serialize},
    yew::{
        format::{Json, Nothing, Toml},
        html,
        prelude::*,
        services::{
            fetch::{FetchService, FetchTask, Request, Response},
            websocket::{WebSocketService, WebSocketStatus, WebSocketTask},
        },
        Component, ComponentLink, Html, ShouldRender,
    },
};

mod gameoflife_client;

pub(crate) mod deps {
    pub use anyhow;
    pub use bincode;
    pub use locutus_game_of_life as gameoflife;
    pub use serde;
    pub use serde_json;
    pub use yew;
}
use self::gameoflife_client::GameOfLifeClient;

pub enum Msg {
    Repaint,
    Toggle,
    ChildClicked(u32),
}

pub struct Model {
    link: ComponentLink<Model>,
}

impl Model {}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(
        _: Self::Properties,
        link: ComponentLink<Self>,
    ) -> Self {
        Model { link }
    }

    fn update(
        &mut self,
        msg: Self::Message,
    ) -> ShouldRender {
        true
    }

    fn change(
        &mut self,
        _: Self::Properties,
    ) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let controller = || {
            html! {
                    <GameOfLifeClient />

            }
        };

        html! {
            <div>
            { controller() }
            </div>
        }
    }
}
