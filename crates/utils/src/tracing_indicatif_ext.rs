#[macro_export]
#[cfg(feature = "tracing_indicatif_ext")]
macro_rules! indicatif_dbg {
	() => {
		$crate::tracing_indicatif::indicatif_eprintln!(
			"[{}:{}:{}]",
			::core::file!(),
			::core::line!(),
			::core::column!(),
		)
	};
	($val:expr $(,)?) => {
		match $val {
			tmp => {
				$crate::tracing_indicatif::indicatif_eprintln!(
					"[{}:{}:{}] {} = {:#?}",
					::core::file!(),
					::core::line!(),
					::core::column!(),
                    ::core::stringify!($val),
                    &&tmp as &dyn ::core::fmt::Debug,
				);
                tmp
			}
		}
	};
    ($($val:expr),+ $(,)?) => {
        ($($crate::indicatif_dbg!($val)),+,)
    };
}
