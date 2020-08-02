use crate::deps::{
    crossbeam::channel::{self, Receiver, Sender},
    gameoflife,
    locutus_actor::{self as actor, Actor},
    parking_lot::{Mutex, MutexGuard},
    serde,
    tracing::info,
};
use std::fmt;
use std::{sync::Arc, time::Duration};

pub struct GameOfLife {
    id: actor::Id,
    game: Mutex<gameoflife::Simulation>,
    tx: Sender<gameoflife::Message>,
    rx: Receiver<gameoflife::Message>,
}

impl GameOfLife {
    pub fn new() -> Self {
        let (tx, rx) = channel::unbounded::<gameoflife::Message>();
        GameOfLife {
            id: crate::deps::locutus_actor::Id::random(),
            game: Mutex::new(gameoflife::Simulation::new()),
            tx,
            rx,
        }
    }
}

impl<'a> Actor<'a> for GameOfLife {
    type Message = gameoflife::Message;
    type State = &'a Mutex<gameoflife::Simulation>;

    fn send(
        &self,
        message: Self::Message,
    ) -> Result<(), Box<dyn actor::Error>> {
        self.tx
            .send(message)
            .map_err(|err| gameoflife::Error::Unknown)
            .map_err(|err| err.into())
    }

    fn on_tick(&self) -> Result<(), Box<dyn actor::Error>> {
        self.rx
            .recv()
            .map_err(|err| gameoflife::Error::Unknown)
            .and_then(|msg| self.game.lock().update(msg))
            .map_err(|err| err.into())
    }

    fn state(&'a self) -> Self::State {
        &self.game
    }

    fn id(&self) -> &actor::Id {
        &self.id
    }

    fn name(&self) -> &'static str {
        "GameOfLife"
    }
}

impl fmt::Debug for GameOfLife {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        f.debug_struct("GameOfLife").field("id", &self.id()).finish()
    }
}
