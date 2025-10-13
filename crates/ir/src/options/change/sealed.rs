pub trait PrimitiveSealed {}

impl PrimitiveSealed for u8 {}
impl PrimitiveSealed for i8 {}

pub trait MarkerSealed {}

impl MarkerSealed for super::Factor {}
impl MarkerSealed for super::Value {}
