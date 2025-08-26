use std::sync::{Arc, Mutex};

#[salsa::db]
#[derive(Clone)]
pub struct CalcDatabase {
	storage: salsa::Storage<Self>,
	logs: Arc<Mutex<Vec<String>>>,
}

impl CalcDatabase {
    pub fn logs(&self) -> Vec<String> {
        self.logs.lock().unwrap().clone()
    }
}

#[salsa::db]
impl salsa::Database for CalcDatabase {}

impl Default for CalcDatabase {
	fn default() -> Self {
		let logs: Arc<Mutex<Vec<String>>> = Arc::default();
		Self {
			storage: salsa::Storage::new(Some(Box::new({
                let logs = logs.clone();
				move |event| {
					eprintln!("Event: {event:?}");
					let logs = &mut *logs.lock().unwrap();
					if let salsa::EventKind::WillExecute { .. } = event.kind {
						logs.push(format!("Event: {event:?}"));
					}
				}
			}))),
			logs,
		}
	}
}
