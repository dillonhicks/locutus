use crate::deps::{
    anyhow::Error,
    gameoflife,
    serde::{
        Deserialize,
        Serialize,
    },
    wasm_bindgen::{
        JsCast,
        JsValue,
    },
    web_sys::{
        CanvasRenderingContext2d as RenderingContext,
        HtmlCanvasElement,
    },
    yew::{
        format::{
            Bincode,
            Json,
            Nothing,
            Toml,
        },
        html,
        prelude::*,
        services::{
            fetch::{
                FetchService,
                FetchTask,
                Request,
                Response,
            },
            websocket::{
                WebSocketService,
                WebSocketStatus,
                WebSocketTask,
            },
            IntervalService,
            Task,
        },
        Component,
        ComponentLink,
        Html,
        ShouldRender,
    },
};

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
    link:     ComponentLink<GameOfLifeClient>,
    data:     Option<gameoflife::Simulation>,
    ws:       Option<WebSocketTask>,
    canvas:   Option<HtmlCanvasElement>,
    ctx:      Option<RenderingContext>,
    node_ref: NodeRef,
}

impl GameOfLifeClient {
    fn render(
        &mut self,
        last_data: Option<&gameoflife::Simulation>,
    ) {
        const CELL_SIZE: f64 = 9.0;
        const PAD: f64 = 1.0;

        let ctx: &RenderingContext = self.ctx.as_ref().expect("Canvas Rendering Context not initialized!");
        let data: &gameoflife::Simulation = self.data.as_ref().expect("Simulation data not initialized!");

        let should_render = |idx: usize, cell: &gameoflife::Cellule, alive: bool| {
            alive == cell.alive()
                && last_data
                    .map(|d: &gameoflife::Simulation| d.cellules()[idx].alive() != cell.alive())
                    .unwrap_or(true)
        };

        ctx.set_fill_style(&JsValue::from_str("green"));

        let mut x = 0.0;
        let mut y = 0.0;
        let mut idx = 0usize;

        for line in data.cellules().chunks(data.width()) {
            for &cell in line {
                if should_render(idx, &cell, true) {
                    ctx.fill_rect(x, y, CELL_SIZE, CELL_SIZE);
                }
                x += CELL_SIZE + PAD;
                idx += 1;
            }
            y += CELL_SIZE + PAD;
            x = 0.0;
        }

        ctx.set_fill_style(&JsValue::from_str("gray"));

        let mut x = 0.0;
        let mut y = 0.0;
        let mut idx = 0usize;

        for line in data.cellules().chunks(data.width()) {
            for &cell in line {
                if should_render(idx, &cell, false) {
                    ctx.fill_rect(x, y, CELL_SIZE, CELL_SIZE);
                }
                x += CELL_SIZE + PAD;
                idx += 1;
            }
            y += CELL_SIZE + PAD;
            x = 0.0;
        }
    }
}

impl Component for GameOfLifeClient {
    type Message = Msg;
    type Properties = Props;

    fn create(
        _props: Self::Properties,
        link: ComponentLink<Self>,
    ) -> Self {
        GameOfLifeClient {
            link,
            data: None,
            ws: None,
            canvas: None,
            ctx: None,
            node_ref: NodeRef::default(),
        }
    }

    fn rendered(
        &mut self,
        _first_render: bool,
    ) {
        // Once rendered, store references for the canvas and GL context. These can be used for
        // resizing the rendering area when the window or canvas element are resized, as well as
        // for making GL calls.

        let canvas = self.node_ref.cast::<HtmlCanvasElement>().unwrap();

        let ctx: RenderingContext = canvas.get_context("2d").unwrap().unwrap().dyn_into().unwrap();

        self.canvas = Some(canvas);
        self.ctx = Some(ctx);
    }

    fn update(
        &mut self,
        msg: Self::Message,
    ) -> ShouldRender {
        match msg {
            Msg::WsAction(action) => {
                match action {
                    WsAction::Connect => {
                        let callback = self.link.callback(|Bincode(data)| Msg::WsReady(data));
                        let notification = self.link.callback(|status| {
                            match status {
                                WebSocketStatus::Opened => Msg::Ignore,
                                WebSocketStatus::Closed | WebSocketStatus::Error => WsAction::Lost.into(),
                            }
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
                        self.data = None;
                    }
                    WsAction::Lost => {
                        self.ws = None;
                    }
                }
            }
            Msg::WsReady(response) => {
                log::info!("{:?}", response);
                let mut last_data = response.ok();
                std::mem::swap(&mut last_data, &mut self.data);
                self.render(last_data.as_ref());
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
                <p> {  self.data.as_ref().map(|sim| sim.state().as_str()).unwrap_or("not running") }</p>
              <canvas width="850" height="640" ref={self.node_ref.clone()} />
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
