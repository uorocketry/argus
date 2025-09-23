use core::str::FromStr;

use heapless::String;
use serde::{Deserialize, Serialize};
use serde_csv_core::{Reader, Writer};

use crate::sd::types::Line;

pub trait SerializeCSV: Serialize + for<'d> Deserialize<'d> {
	fn get_csv_header() -> Line;
	fn from_csv_line(line: &Line) -> Self {
		let mut reader = Reader::<255>::new();
		let (record, _n) = reader.deserialize::<Self>(line.as_bytes()).unwrap();
		record
	}
	fn to_csv_line(&self) -> Line {
		let mut writer = Writer::new();
		let mut line = [0u8; 255];
		writer.serialize(&self, &mut line).unwrap();
		let line_str = core::str::from_utf8(&line).unwrap();

		String::from_str(line_str).unwrap()
	}
}
