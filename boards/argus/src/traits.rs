//! This module should be refactored out.
use crate::state_machine::{StateMachineContext, States};
use messages::state::DeviceState;

pub struct Context {}

impl StateMachineContext for Context {}

impl From<States> for DeviceState {
    fn from(value: States) -> Self {
        match value {
            States::Idle => DeviceState::Idle,
            States::Calibration => DeviceState::Calibration,
            States::Collection => DeviceState::Collection,
            States::Init => DeviceState::Init,
            States::Fault => DeviceState::Fault,
        }
    }
}
