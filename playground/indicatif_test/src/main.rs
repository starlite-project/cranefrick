use std::time::Duration;

use futures::stream::{
	StreamExt, {self},
};
use rand::Rng;
use tracing::{Span, info_span, instrument};
use tracing_indicatif::{IndicatifLayer, span_ext::IndicatifSpanExt, style::ProgressStyle};
use tracing_subscriber::{
	EnvFilter, fmt, layer::SubscriberExt, prelude::*, util::SubscriberInitExt,
};

#[tokio::main]
async fn main() {
	install_tracing();

	let header_span = info_span!("header");
	header_span.pb_set_style(
		&ProgressStyle::with_template(
			"{span_child_prefix}{spinner} {span_name}({span_fields}) [{elapsed_precise}] [{bar:38}] ({eta})",
		)
		.unwrap()
		.progress_chars("#>-"),
	);
	header_span.pb_set_length(20);

	let header_span_enter = header_span.enter();

	let res: u64 = stream::iter((0..20).map(|val| async move {
		let res = do_work(val).await;
		Span::current().pb_inc(1);
		res
	}))
	.buffer_unordered(5)
	.collect::<Vec<u64>>()
	.await
	.into_iter()
	.sum();

	drop(header_span_enter);
	drop(header_span);

	println!("final result: {res}");
}

#[instrument]
async fn do_sub_work(val: u64) -> u64 {
	let sleep_time = rand::rng().random_range(Duration::from_secs(3)..Duration::from_secs(5));
	tokio::time::sleep(sleep_time).await;

	val + 1
}

#[instrument]
async fn do_work(mut val: u64) -> u64 {
	let sleep_time = rand::rng().random_range(Duration::from_secs(1)..Duration::from_secs(3));
	tokio::time::sleep(sleep_time).await;

	val = do_sub_work(val).await;

	val + 1
}

fn install_tracing() {
	let indicatif_layer = IndicatifLayer::new().with_progress_style(
		ProgressStyle::with_template(
			"{span_child_prefix}{spinner} {span_name}({span_fields}) [{elapsed_precise}]",
		)
		.unwrap()
		.progress_chars("#>-"),
	);
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
