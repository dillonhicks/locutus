use std::time::Duration;

use crate::deps::{
    rand::{thread_rng, Rng},
    serde, thiserror,
    tracing::info,
};

#[cfg(feature = "actor")]
use crate::deps::locutus_actor as actor;
use std::fmt;

pub(crate) mod deps {
    pub use rand;
    pub use serde;
    pub use thiserror;
    pub use tracing;

    #[cfg(feature = "actor")]
    pub use locutus_actor;
}

fn wrap(
    coord: isize,
    range: isize,
) -> usize {
    let result = if coord < 0 {
        coord + range
    } else if coord >= range {
        coord - range
    } else {
        coord
    };
    result as usize
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unknown error")]
    Unknown,

    #[error("the game is over")]
    GameOver,

    #[error("invalid state transition from {from:?} -> {to:?}")]
    StateTransition { from: State, to: State },
}

impl Error {
    pub fn may_recover(&self) -> bool {
        match self {
            Error::Unknown => false,
            Error::GameOver => false,
            Error::StateTransition { .. } => true,
        }
    }

    pub fn may_retry(&self) -> bool {
        match self {
            Error::Unknown => false,
            Error::GameOver => false,
            Error::StateTransition { from: _, to: _ } => false,
        }
    }
}

#[cfg(feature = "actor")]
impl actor::Error for Error {
    fn recoverable(&self) -> bool {
        self.may_recover()
    }

    fn retryable(&self) -> bool {
        self.may_retry()
    }
}

#[cfg(feature = "actor")]
impl Into<Box<dyn actor::Error>> for Error {
    fn into(self) -> Box<dyn actor::Error> {
        Box::new(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum State {
    Starting,
    Running,
    Pausing,
    Ended,
}

impl std::default::Default for State {
    fn default() -> State {
        State::Starting
    }
}

impl State {
    pub fn try_transition(
        &mut self,
        next: State,
    ) -> Result<(), Error> {
        use State::*;
        match (*self, next) {
            (Starting, Starting)
            | (Starting, Running)
            | (Starting, Pausing)
            | (Starting, Ended)
            | (Running, Running)
            | (Running, Ended)
            | (Running, Pausing)
            | (Pausing, Running)
            | (Pausing, Ended)
            | (Pausing, Pausing)
            | (Ended, Ended) => {
                *self = next;
            }
            (Running, Starting) | (Pausing, Starting) | (Ended, Starting) | (Ended, Running) | (Ended, Pausing) => {
                return Err(Error::StateTransition { from: *self, to: next });
            }
        }

        Ok(())
    }

    pub fn paused(&self) -> bool {
        use State::*;

        if let Pausing = self {
            true
        } else {
            false
        }
    }

    pub fn start(&self) -> bool {
        use State::*;

        if let Starting = self {
            true
        } else {
            false
        }
    }

    pub fn run(&self) -> bool {
        use State::*;

        if let Running = self {
            true
        } else {
            false
        }
    }

    pub fn ended(&self) -> bool {
        use State::*;

        if let Ended = self {
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Message {
    Random,
    Start,
    Step,
    Reset,
    Stop,
    ToggleCellule(usize),
    Tick,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum LifeState {
    Alive = 1,
    Dead = 0,
}

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Cellule {
    life_state: LifeState,
}

impl Cellule {
    pub fn set_alive(&mut self) {
        self.life_state = LifeState::Alive;
    }

    pub fn set_dead(&mut self) {
        self.life_state = LifeState::Dead;
    }

    pub fn alive(self) -> bool {
        self.life_state == LifeState::Alive
    }

    pub fn count_alive_neighbors(neighbors: &[Cellule]) -> usize {
        neighbors.iter().filter(|n| n.alive()).count()
    }

    pub fn alone(neighbors: &[Cellule]) -> bool {
        Self::count_alive_neighbors(neighbors) < 2
    }

    pub fn overpopulated(neighbors: &[Cellule]) -> bool {
        Self::count_alive_neighbors(neighbors) > 3
    }

    pub fn can_be_revived(neighbors: &[Cellule]) -> bool {
        Self::count_alive_neighbors(neighbors) == 3
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Simulation {
    state: State,
    ticks: usize,
    cellules: Vec<Cellule>,
    cellules_width: usize,
    cellules_height: usize,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            state: State::default(),
            ticks: 0,
            cellules: vec![
                Cellule {
                    life_state: LifeState::Dead,
                };
                53 * 40
            ],
            cellules_width: 53,
            cellules_height: 40,
        }
    }

    pub fn ticks(&self) -> usize {
        self.ticks
    }

    pub fn cellules(&self) -> &[Cellule] {
        &self.cellules[..]
    }

    pub fn height(&self) -> usize {
        self.cellules_height
    }

    pub fn width(&self) -> usize {
        self.cellules_width
    }

    pub fn random_mutate(&mut self) {
        for cellule in self.cellules.iter_mut() {
            if thread_rng().gen() {
                cellule.set_alive();
            } else {
                cellule.set_dead();
            }
        }
    }

    fn reset(&mut self) {
        for cellule in self.cellules.iter_mut() {
            cellule.set_dead();
        }
    }

    fn step(&mut self) {
        self.ticks += 1;
        let mut to_dead = Vec::new();
        let mut to_live = Vec::new();
        for row in 0..self.cellules_height {
            for col in 0..self.cellules_width {
                let neighbors = self.neighbors(row as isize, col as isize);

                let current_idx = self.row_col_as_idx(row as isize, col as isize);
                if self.cellules[current_idx].alive() {
                    if Cellule::alone(&neighbors) || Cellule::overpopulated(&neighbors) {
                        to_dead.push(current_idx);
                    }
                } else if Cellule::can_be_revived(&neighbors) {
                    to_live.push(current_idx);
                }
            }
        }
        to_dead.iter().for_each(|idx| self.cellules[*idx].set_dead());
        to_live.iter().for_each(|idx| self.cellules[*idx].set_alive());
    }

    fn neighbors(
        &self,
        row: isize,
        col: isize,
    ) -> [Cellule; 8] {
        [
            self.cellules[self.row_col_as_idx(row + 1, col)],
            self.cellules[self.row_col_as_idx(row + 1, col + 1)],
            self.cellules[self.row_col_as_idx(row + 1, col - 1)],
            self.cellules[self.row_col_as_idx(row - 1, col)],
            self.cellules[self.row_col_as_idx(row - 1, col + 1)],
            self.cellules[self.row_col_as_idx(row - 1, col - 1)],
            self.cellules[self.row_col_as_idx(row, col - 1)],
            self.cellules[self.row_col_as_idx(row, col + 1)],
        ]
    }

    fn row_col_as_idx(
        &self,
        row: isize,
        col: isize,
    ) -> usize {
        let row = wrap(row, self.cellules_height as isize);
        let col = wrap(col, self.cellules_width as isize);

        row * self.cellules_width + col
    }

    fn toggle_cellule(
        &mut self,
        idx: usize,
    ) {
        let cellule = self.cellules.get_mut(idx).unwrap();
        if cellule.life_state == LifeState::Alive {
            cellule.life_state = LifeState::Dead
        } else {
            cellule.life_state = LifeState::Alive
        };
    }

    pub fn update(
        &mut self,
        msg: Message,
    ) -> Result<(), Error> {
        use State::*;
        if self.state.ended() {
            return Err(Error::GameOver);
        }

        match msg {
            Message::Random => {
                self.random_mutate();
                info!("Random");
            }
            Message::Start => {
                self.state.try_transition(Running)?;
                info!("Start");
            }
            Message::Step => {
                self.step();
            }
            Message::Reset => {
                self.reset();
                info!("Reset");
            }
            Message::Stop => {
                self.state.try_transition(Pausing)?;
                info!("Stop");
            }
            Message::ToggleCellule(idx) => {
                self.toggle_cellule(idx);
            }
            Message::Tick => {
                if self.state.run() {
                    self.step();
                }
            }
            Message::End => {
                self.state.try_transition(Ended)?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for Simulation {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        for line in self.cellules.as_slice().chunks(self.width()) {
            for &cell in line {
                let symbol = if cell.alive() { '◼' } else { '◻' };
                write!(f, "{}", symbol)?;
            }
            write!(f, "\n")?;
        }

        Ok(())
    }
}
