use anyhow::Result;
use befunge_test::execute;

fn main() -> Result<()> {
	execute(&[], &[], &mut [], &mut [], &mut 0)?;

	Ok(())
}
