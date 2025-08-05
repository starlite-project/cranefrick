mod if_lets;

struct Context;
impl if_lets::Context for Context {
    fn A(&mut self, a: u32, b: u32) -> Option<u32> {
        Some(a + b)
    }

    fn B(&mut self, value: u32) -> Option<(u32, u32)> {
        Some((value, value + 1))
    }
}

fn main() {
    if_lets::constructor_C(&mut Context, 1, 2, 3, 4);
}
