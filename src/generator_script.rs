//! Вшитый генератор параметризованных челленджей (adversarial loop).
pub const GENERATOR_PY: &str = include_str!("../tools/challenge_generator.py");

#[cfg(test)]
mod tests {
    #[test]
    fn generator_script_is_embedded() {
        assert!(super::GENERATOR_PY.contains("TEMPLATES"));
        assert!(super::GENERATOR_PY.len() > 3000);
    }
}
