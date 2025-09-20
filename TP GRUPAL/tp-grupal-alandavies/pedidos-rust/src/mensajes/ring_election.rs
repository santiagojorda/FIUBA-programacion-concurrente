use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const TIPO_ELECTION: u8 = 0;
pub const TIPO_COORDINATOR: u8 = 1;

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
pub struct RingElection {
    pub tipo: u8,
    pub puerto_original: u16,
    pub puertos: Vec<u16>,
}

impl fmt::Display for RingElection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Ring_Election(tipo: {}, puerto_original: {}, puertos: {:?})",
            self.tipo, self.puerto_original, self.puertos
        )
    }
}

impl RingElection {
    pub fn new(puerto_original: u16) -> Self {
        Self {
            tipo: TIPO_ELECTION,
            puerto_original,
            puertos: Vec::from([puerto_original]),
        }
    }

    pub fn cambiar_a_coordinator(&mut self) {
        self.tipo = TIPO_COORDINATOR;
    }

    pub fn tipo(&self) -> u8 {
        self.tipo
    }

    pub fn puerto_original(&self) -> u16 {
        self.puerto_original
    }

    pub fn puerto_maximo(&self) -> Option<u16> {
        self.puertos.iter().max().cloned()
    }

    pub fn agregar_puerto(&mut self, puerto: u16) {
        self.puertos.push(puerto);
    }
}
