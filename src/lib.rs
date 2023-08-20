use crate::assets::FREQUENCIES;
use crate::constant::{ENCODING_MARKS, IANA_SUPPORTED};
use encoding::all::encodings;
use encoding::label;
use regex::Regex;
mod assets;
mod constant;

pub fn add(left: usize, right: usize) -> usize {
    println!("{:?}", *IANA_SUPPORTED);
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
