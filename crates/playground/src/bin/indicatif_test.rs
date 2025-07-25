use std::{thread, time::Duration};

use tracing::{info, info_span};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tracing::instrument]
fn main() {
	install_tracing();

	let span = info_span!("run");

	info!("starting progress bar");

	span.in_scope(|| {
		for i in 0..1000 {
			thread::sleep(Duration::from_millis(5));
			if matches!(i % 10, 0) {
				info!("finished {i}");
			}
		}
	});
}

fn install_tracing() {
	let indicatif_layer = IndicatifLayer::new();
	let filter_layer = EnvFilter::new("info");
	let fmt_layer = fmt::layer()
		.with_target(false)
		.compact()
		.with_writer(indicatif_layer.get_stderr_writer())
		.with_filter(filter_layer);

	tracing_subscriber::registry()
		.with(fmt_layer)
		.with(indicatif_layer)
		.init();
}
