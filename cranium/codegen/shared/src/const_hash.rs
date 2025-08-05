#[must_use]
pub fn simple_hash(s: &str) -> usize {
	let mut h = 5381u32;
	for c in s.chars() {
		h = (h ^ c as u32).wrapping_add(h.rotate_right(6));
	}

	h as usize
}

#[cfg(test)]
mod tests {
    use super::simple_hash;

    #[test]
    fn basic() {
        assert_eq!(simple_hash("Hello"), 0x2fa7_0c01);
        assert_eq!(simple_hash("world"), 0x5b0c_31d5);
    }
}
