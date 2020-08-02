pub(crate) mod deps {
    #[cfg(feature = "serde")]
    pub use serde1 as serde;

    #[cfg(feature = "uuid")]
    pub use uuid1 as uuid;
}

use std::fmt;

#[cfg(feature = "serde")]
use crate::deps::serde::{Deserialize, Serialize};

pub trait Error: std::error::Error {
    fn recoverable(&self) -> bool;
    fn retryable(&self) -> bool;
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Hash)]
#[cfg_attr(not(feature = "uuid"), derive(Debug))]
pub struct Id(std::num::NonZeroU128);

impl Id {
    pub fn new(id: u128) -> Option<Self> {
        std::num::NonZeroU128::new(id).map(Id)
    }

    #[cfg(feature = "uuid")]
    pub fn random() -> Self {
        let uid = crate::deps::uuid::Uuid::new_v4().as_u128();
        Self(unsafe { std::num::NonZeroU128::new_unchecked(uid) })
    }

    pub const fn into_inner(&self) -> u128 {
        self.0.get()
    }
}

#[cfg(feature = "uuid")]
impl fmt::Debug for Id {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(
            f,
            "Id({})",
            crate::deps::uuid::Uuid::from_u128(self.0.get()).to_hyphenated_ref()
        )
    }
}

#[cfg(feature = "uuid")]
impl fmt::Display for Id {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        crate::deps::uuid::Uuid::from_u128(self.0.get())
            .to_hyphenated_ref()
            .fmt(f)
    }
}

pub trait Actor<'a> {
    type Message;
    #[cfg(feature = "serde")]
    type State: 'a + Serialize;
    #[cfg(not(feature = "serde"))]
    type State: 'a;

    fn name(&self) -> &'static str;
    fn id(&self) -> &Id;

    fn send(
        &self,
        message: Self::Message,
    ) -> Result<(), Box<dyn Error>>;
    fn on_tick(&self) -> Result<(), Box<dyn Error>>;
    fn state(&'a self) -> Self::State;
}
