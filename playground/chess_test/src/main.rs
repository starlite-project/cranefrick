use color_eyre::Result;
use shakmaty::{Chess, Move, Position, Role, Square};

fn main() -> Result<()> {
	color_eyre::install()?;

	let pos = Chess::default();

	let pos = pos.play(Move::Normal {
		role: Role::Pawn,
		from: Square::E2,
		to: Square::E4,
		capture: None,
		promotion: None,
	})?;

	let legals = pos.legal_moves();

	println!("{legals:?}");

	Ok(())
}
