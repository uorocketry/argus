use core::future::Future;

use defmt::{error, info};
use embassy_futures::select::{select, Either};

use crate::{
	state_machine::types::{Context, Events, StateMachine, StateReceiver, StateWatch, States},
	utils::types::AsyncMutex,
};

// Current state cannot be stored on the state machine itself, as we need to be able to
// reference it from multiple async tasks. So we store it in a static watch globally instead.
static CURRENT_STATE: StateWatch = StateWatch::new();

// Encapsulates the logic for managing the state machine
pub struct StateMachineOrchestrator {
	state_machine: StateMachine<Context>,
}
impl Default for StateMachineOrchestrator {
	fn default() -> Self {
		Self::new()
	}
}

impl StateMachineOrchestrator {
	pub fn new() -> Self {
		Self {
			state_machine: StateMachine::new(Context {}),
		}
	}

	pub fn dispatch_event(
		&mut self,
		event: Events,
	) {
		let previous_state = self.state_machine.state().clone();
		let result = self.state_machine.process_event(event.clone());
		match result {
			Ok(state) => {
				CURRENT_STATE.sender().send(state.clone());
				info!("State changed from {:?} to {:?} due to event {:?}", previous_state, state, event);
			}
			Err(_) => {
				error!("Invalid event dispatched in state {:?}: {:?}", previous_state, event);
			}
		}
	}
}

// Encapsulates the logic needed by a worker task
pub struct StateMachineWorker {
	receiver: StateReceiver,
	orchestrator: &'static AsyncMutex<StateMachineOrchestrator>,
}
impl StateMachineWorker {
	pub fn new(orchestrator: &'static AsyncMutex<StateMachineOrchestrator>) -> Self {
		Self {
			receiver: CURRENT_STATE.receiver().unwrap(),
			orchestrator,
		}
	}

	pub async fn run_while<Err, Act, Fut>(
		&mut self,
		desired_state: States,
		mut action: Act,
	) -> Result<(), Err>
	where
		Act: FnMut(&'static AsyncMutex<StateMachineOrchestrator>) -> Fut,
		Fut: Future<Output = Result<(), Err>>, {
		// Runs indefinitely
		loop {
			// debug!("Waiting for state change...");

			// Wait until we're in the desired state
			self.receiver.changed_and(|_state| *_state == desired_state).await;

			// We're now in the desired state, run the action until the state changes
			loop {
				// debug!("In desired state {:?}, running action...", desired_state);
				// Race between state change or action completion
				let state_changed = self.receiver.changed();
				let action_finished = action(self.orchestrator);

				match select(state_changed, action_finished).await {
					Either::First(_) => {
						break;
					}
					Either::Second(_) => {
						if self.receiver.get().await != desired_state {
							// debug!("State changed while action was running, stopping action.");
							break;
						}
					}
				}
			}
		}
	}
}
