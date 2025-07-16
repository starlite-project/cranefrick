use cranefrick_hir::BrainHir;
use serde::{Deserialize, Serialize};

/// Mid-level intermediate representation. Not 1 to 1 for it's brainfuck equivalent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrainMir {
	ChangeCell(i8),
	MovePtr(i64),
	SetCell(i8),
	GetInput,
	PutOutput,
	StartLoop,
	EndLoop,
}

impl From<BrainHir> for BrainMir {
    fn from(value: BrainHir) -> Self {
        match value {
            BrainHir::IncrementCell => Self::ChangeCell(1),
            BrainHir::DecrementCell => Self::ChangeCell(-1),
            BrainHir::MovePtrLeft => Self::MovePtr(-1),
            BrainHir::MovePtrRight => Self::MovePtr(1),
            BrainHir::GetInput => Self::GetInput,
            BrainHir::PutOutput => Self::PutOutput,
            BrainHir::StartLoop => Self::StartLoop,
            BrainHir::EndLoop => Self::EndLoop,
        }
    }
}
