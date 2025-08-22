use color_eyre::Result;
use inkwell::intrinsics::Intrinsic;

fn main() -> Result<()> {
    color_eyre::install()?;

    let intrinsic = Intrinsic::find("llvm.assume");

    dbg!(intrinsic);

    Ok(())
}
