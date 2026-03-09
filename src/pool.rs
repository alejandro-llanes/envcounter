use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Pool {
    Counter(Counter),
    Uuid(UuidPool),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Counter {
    pub value: i64,
    pub step: i64,
    pub initial: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UuidPool;

impl Counter {
    pub fn new(step: i64, initial: i64) -> Self {
        Self {
            value: initial,
            step,
            initial,
        }
    }

    /// Return current value and increment by step.
    pub fn read(&mut self) -> i64 {
        let current = self.value;
        self.value += self.step;
        current
    }

    /// Return current value without incrementing.
    pub fn seek(&self) -> i64 {
        self.value
    }

    /// Reset value back to initial.
    pub fn reset(&mut self) {
        self.value = self.initial;
    }
}

impl UuidPool {
    pub fn new() -> Self {
        Self
    }

    /// Generate a new random UUID-v4.
    pub fn read(&self) -> String {
        Uuid::new_v4().to_string()
    }
}

impl Pool {
    pub fn as_counter_mut(&mut self) -> Option<&mut Counter> {
        match self {
            Pool::Counter(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_counter(&self) -> Option<&Counter> {
        match self {
            Pool::Counter(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_uuid(&self) -> Option<&UuidPool> {
        match self {
            Pool::Uuid(u) => Some(u),
            _ => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Pool::Counter(_) => "counter",
            Pool::Uuid(_) => "uuid",
        }
    }
}
