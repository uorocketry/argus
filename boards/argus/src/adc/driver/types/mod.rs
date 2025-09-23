pub mod analog_channel;
pub mod command;
pub mod data_rate;
pub mod filter;
pub mod gain;
pub mod reference_range;
pub mod register;

pub use analog_channel::*;
pub use command::*;
pub use data_rate::*;
pub use filter::*;
pub use gain::*;
pub use reference_range::*;
pub use register::*;

pub type Voltage = f32;
