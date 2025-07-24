use std::sync::Once;

pub fn init_tracing() {
	static INITIALIZED: Once = Once::new();

	INITIALIZED.call_once(|| {
		tracing_subscriber::fmt()
			.with_test_writer()
			.with_max_level(tracing::Level::TRACE)
			.pretty()
			.with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
			.init();
	});
}
