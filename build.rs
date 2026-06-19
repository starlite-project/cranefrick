use std::error::Error as StdError;

fn main() -> Result<(), Box<dyn StdError>> {
	vcpkg::find_package("libxml2")?;

	Ok(())
}
