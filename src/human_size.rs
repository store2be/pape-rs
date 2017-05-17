use std::str::FromStr;
use regex::Regex;

/// Represents a size as a number of bytes
#[derive(Clone, Copy, Debug)]
pub struct Bytes(pub u32);

fn scale(quantity: u32, unit: &str) -> u32 {
    match unit {
        "K" => quantity * 1_000,
        "M" => quantity * 1_000_000,
        "G" => quantity * 1_000_000_000,
        _ => unreachable!(),
    }
}

impl FromStr for Bytes {
    type Err = ();

    fn from_str(src: &str) -> Result<Bytes, ()> {
        lazy_static! {
            static ref HUMAN_READABLE: Regex = Regex::new(r"^\s*(\d+)([GKM])?\s*$").unwrap();
        }

        match HUMAN_READABLE.captures(src) {
            Some(captures) => {
                let quantity: u32 = captures.get(1).unwrap().as_str().parse().unwrap();
                match captures.get(2) {
                    Some(unit) => Ok(Bytes(scale(quantity, unit.as_str()))),
                    None => Ok(Bytes(quantity)),
                }
            },
            None => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};
    use super::*;

    #[test]
    fn test_bytes_from_str() {
        let assert = |src, expected| {
            assert_eq!(Bytes::from_str(src).map(|by| by.0), expected);
        };

        assert("10M", Ok(10_000_000));
        assert("10", Ok(10));
        assert("33", Ok(33));
        assert("1G", Ok(1_000_000_000));
    }

    #[derive(Clone, Debug)]
    struct Unit(char);

    impl Arbitrary for Unit {
        fn arbitrary<G: Gen>(gen: &mut G) -> Unit {
            let choices = ['K', 'M', 'G'];
            Unit(choices[(gen.size() / usize::max_value()) as usize])
        }
    }

    quickcheck! {
        fn human_size_with_arbitrary_input_never_panics(src: String) -> bool {
            match Bytes::from_str(&src) {
                Ok(_) => true,
                Err(()) => true,
            }

        }

        fn human_size_without_unit_roundtrips(quantity: u32) -> bool {
            let src = format!("{}", quantity);
            Bytes::from_str(&src).unwrap().0 == quantity
        }

        fn human_size_with_unit_roundtrips(quantity: u32, unit: Unit) -> bool {
            let src = format!("{}{}", quantity, unit.0.to_string());
            Bytes::from_str(&src).unwrap().0 == scale(quantity, &unit.0.to_string())
        }
    }
}
