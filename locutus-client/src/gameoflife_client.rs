use crate::deps::yew::format::Binary;
use crate::deps::{
    anyhow::Error,
    gameoflife,
    serde::{Deserialize, Serialize},
    yew::{
        format::Bincode,
        format::{Json, Nothing, Toml},
        html,
        prelude::*,
        services::{
            fetch::{FetchService, FetchTask, Request, Response},
            websocket::{WebSocketService, WebSocketStatus, WebSocketTask},
            IntervalService, Task,
        },
        Component, ComponentLink, Html, ShouldRender,
    },
};
use std::time::Duration;

pub(crate) type AsBinary = bool;

pub enum Format {
    Json,
    Toml,
}

pub enum WsAction {
    Connect,
    SendData(AsBinary),
    Disconnect,
    Lost,
}

pub enum Msg {
    WsAction(WsAction),
    WsReady(Result<gameoflife::Simulation, Error>),
    Ignore,
}

impl From<WsAction> for Msg {
    fn from(action: WsAction) -> Self {
        Msg::WsAction(action)
    }
}

/// This type is used to parse data from `./static/data.json` file and
/// have to correspond the data layout from that file.
#[derive(Deserialize, Debug)]
pub struct DataFromFile {
    value: u32,
}

/// This type is used as a request which sent to websocket connection.
#[derive(Serialize, Debug)]
struct WsRequest {
    value: u32,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {}

pub struct GameOfLifeClient {
    link: ComponentLink<GameOfLifeClient>,
    fetching: bool,
    data: Option<gameoflife::Simulation>,
    ft: Option<FetchTask>,
    ws: Option<WebSocketTask>,
}

fn render_simulation(sim: &gameoflife::Simulation) -> Html {
    html! {
        <p> {
        sim
        }</p>
    }
}

impl GameOfLifeClient {
    fn view_data(&self) -> Html {
        if let Some(value) = &self.data {
            html! {
                <p>{ render_simulation(value) }</p>
            }
        } else {
            html! {
                <p>{ "Data hasn't fetched yet." }</p>
            }
        }
    }
}

impl Component for GameOfLifeClient {
    type Message = Msg;
    type Properties = Props;

    fn create(
        props: Self::Properties,
        link: ComponentLink<Self>,
    ) -> Self {
        GameOfLifeClient {
            link,
            fetching: false,
            data: None,
            ft: None,
            ws: None,
        }
    }

    fn update(
        &mut self,
        msg: Self::Message,
    ) -> ShouldRender {
        match msg {
            Msg::WsAction(action) => match action {
                WsAction::Connect => {
                    let callback = self.link.callback(|Bincode(data)| Msg::WsReady(data));
                    let notification = self.link.callback(|status| match status {
                        WebSocketStatus::Opened => Msg::Ignore,
                        WebSocketStatus::Closed | WebSocketStatus::Error => WsAction::Lost.into(),
                    });
                    let task = WebSocketService::connect("ws://localhost:9001/", callback, notification).unwrap();
                    self.ws = Some(task);
                }
                WsAction::SendData(binary) => {
                    let request = WsRequest { value: 321 };
                    if binary {
                        self.ws.as_mut().unwrap().send_binary(Json(&request));
                    } else {
                        self.ws.as_mut().unwrap().send(Json(&request));
                    }
                }
                WsAction::Disconnect => {
                    self.ws.take();
                }
                WsAction::Lost => {
                    self.ws = None;
                }
            },
            Msg::WsReady(response) => {
                log::info!("{:?}", response);
                self.data = response.ok();
            }
            Msg::Ignore => {
                return false;
            }
        }
        true
    }

    fn change(
        &mut self,
        _: Self::Properties,
    ) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                { self.view_data() }
                <nav class="menu">
                    <button disabled=self.ws.is_some()
                            onclick=self.link.callback(|_| WsAction::Connect)>
                        { "Start Simulation" }
                    </button>
                    <button disabled=self.ws.is_none()
                            onclick=self.link.callback(|_| WsAction::Disconnect)>
                        { "Terminate Simulation" }
                    </button>
                </nav>
            </div>
        }
    }
}
