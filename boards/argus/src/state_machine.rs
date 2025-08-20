use defmt::info;
use embassy_executor::Spawner;
use embassy_time::Instant;
use messages_prost::argus_state::State;
use messages_prost::prost::Message;
use smlang::statemachine;

use crate::resources::{EVENT_CHANNEL, SD_CHANNEL};

statemachine! {
    transitions: {
        *Init + CalibrationRequested = Calibration,
        Calibration + Calibrated = Idle,
        Idle + CollectionRequested = Collection
    }
}

pub struct Context {}

impl StateMachineContext for Context {}

impl From<States> for State {
    fn from(value: States) -> Self {
        match value {
            States::Calibration => State::Calibration,
            States::Collection => State::Collection,
            States::Idle => State::Idle,
            States::Init => State::Init,
        }
    }
}

#[embassy_executor::task]
pub async fn sm_task(spawner: Spawner, mut state_machine: StateMachine<Context>) {
    info!("State Machine task started.");

    loop {
        if let Ok(event) = EVENT_CHANNEL.try_receive() {
            state_machine.process_event(event);
        }

        match state_machine.state {
            States::Init => {
                let mut buf: [u8; 255] = [0; 255];

                let msg = messages_prost::radio::RadioFrame {
                    node: messages_prost::common::Node::Phoenix.into(),
                    payload: Some(messages_prost::radio::radio_frame::Payload::PhoenixState(
                        State::Init.into(),
                    )),
                    millis_since_start: Instant::now().as_millis(),
                };
                msg.encode_length_delimited(&mut buf.as_mut()).unwrap();
                SD_CHANNEL.send(("state.txt", buf)).await;
            }
            States::Idle => {
                let mut buf: [u8; 255] = [0; 255];

                let msg = messages_prost::radio::RadioFrame {
                    node: messages_prost::common::Node::Phoenix.into(),
                    payload: Some(messages_prost::radio::radio_frame::Payload::PhoenixState(
                        State::Idle.into(),
                    )),
                    millis_since_start: Instant::now().as_millis(),
                };
                msg.encode_length_delimited(&mut buf.as_mut()).unwrap();
                SD_CHANNEL.send(("state.txt", buf)).await;
            }
            States::Calibration => {
                let mut buf: [u8; 255] = [0; 255];

                let msg = messages_prost::radio::RadioFrame {
                    node: messages_prost::common::Node::Phoenix.into(),
                    payload: Some(messages_prost::radio::radio_frame::Payload::PhoenixState(
                        State::Calibration.into(),
                    )),
                    millis_since_start: Instant::now().as_millis(),
                };
                msg.encode_length_delimited(&mut buf.as_mut()).unwrap();
                SD_CHANNEL.send(("state.txt", buf)).await;
            }
            States::Collection => {
                let mut buf: [u8; 255] = [0; 255];

                let msg = messages_prost::radio::RadioFrame {
                    node: messages_prost::common::Node::Phoenix.into(),
                    payload: Some(messages_prost::radio::radio_frame::Payload::PhoenixState(
                        State::Collection.into(),
                    )),
                    millis_since_start: Instant::now().as_millis(),
                };
                msg.encode_length_delimited(&mut buf.as_mut()).unwrap();
                SD_CHANNEL.send(("state.txt", buf)).await;
                info!("Wait For Launch");
            }
        }
    }
}
