#![feature(test)]

use std::{fmt::Display, io::Write};

pub const NUM_COLORS: u8 = 16;
pub const NUM_FIELDS: u32 = 6;
pub type ColorBitmask = u8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Guess<const FIELDS: usize>([u8; FIELDS]);

impl<const FIELDS: usize> Default for Guess<FIELDS> {
    fn default() -> Self {
        Self([0; FIELDS])
    }
}
const NAMES: [&str; 8] = [
    "rot", "grün", "gelb", "blau", "orange", "pink", "weiß", "grau",
];

impl<const FIELDS: usize> Display for Guess<FIELDS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for field in self.0.iter() {
            if first {
                write!(f, "{}", NAMES[*field as usize])?;
            } else {
                write!(f, ", {}", NAMES[*field as usize])?;
            }
            first = false;
        }
        Ok(())
    }
}

impl<const FIELDS: usize> Guess<FIELDS> {
    fn iter<const NUM_COLORS: u8>(&self) -> GuessIterator<FIELDS, NUM_COLORS> {
        GuessIterator {
            current: *self,
            exhausted: false,
        }
    }

    fn is_valid_code(&self) -> bool {
        let mut colors: ColorBitmask = 0;
        for color in self.0 {
            if colors & (1 << color) > 0 {
                return false;
            }
            colors |= 1 << color;
        }
        true
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Evaluation<const FIELDS: usize> {
    correct_color: u32,
    correct_color_and_position: u32,
}

impl<const FIELDS: usize> Evaluation<FIELDS> {
    const MAX_GAUSS: usize = (FIELDS as usize + 2) * (FIELDS + 1) as usize / 2;
    fn lut() -> [usize; FIELDS] {
        std::array::from_fn(|i| (i + 3) * (i + 2) / 2 - 1)
    }
    pub fn to_u32(&self) -> u32 {
        if self.correct_color == FIELDS as u32 {
            return Evaluation::<FIELDS>::MAX_GAUSS as u32 - 1;
        }
        Evaluation::<FIELDS>::MAX_GAUSS as u32 + self.correct_color_and_position
            - 1
            - Evaluation::<FIELDS>::lut()[FIELDS - 1 - self.correct_color as usize] as u32
    }
}

pub struct Entry<const FIELDS: usize> {
    guess: Guess<FIELDS>,
    evaluation: Evaluation<FIELDS>,
}

#[derive(Default)]
pub struct GuessIterator<const FIELDS: usize, const COLORS: u8> {
    current: Guess<FIELDS>,
    exhausted: bool,
}

impl<const FIELDS: usize, const COLORS: u8> Iterator for GuessIterator<FIELDS, COLORS> {
    type Item = Guess<FIELDS>;
    fn next(&mut self) -> Option<Guess<FIELDS>> {
        let old = self.current;
        if self.exhausted {
            return None;
        }
        if self.current.0.into_iter().all(|x| x == COLORS - 1) {
            self.exhausted = true;
        }
        self.current.0[0] += 1;
        for i in 0..(FIELDS - 1) {
            if self.current.0[i] >= COLORS {
                self.current.0[i] = 0;
                self.current.0[i + 1] += 1;
            }
        }
        Some(old)
    }
}

#[derive(Default)]
pub struct CodeIterator<const FIELDS: usize, const COLORS: u8> {
    current: Guess<FIELDS>,
}

impl<const FIELDS: usize, const COLORS: u8> Iterator for CodeIterator<FIELDS, COLORS> {
    type Item = Guess<FIELDS>;

    fn next(&mut self) -> Option<Self::Item> {
        self.current = self
            .current
            .iter::<COLORS>()
            .skip(1)
            .find(|guess| guess.is_valid_code())?;
        Some(self.current)
    }
}

pub trait Solver<const FIELDS: usize> {
    fn guess(&mut self, history: &[Entry<FIELDS>]) -> Guess<FIELDS>;
}

pub fn evaluate<const FIELDS: usize>(
    code: Guess<FIELDS>,
    guess: Guess<FIELDS>,
) -> Evaluation<FIELDS> {
    let mut exact_matches = 0;
    let mut inexact_matches = 0;
    let mut colors: ColorBitmask = 0;

    for color in code.0 {
        colors |= 1 << color
    }

    for i in 0..FIELDS {
        if code.0[i] == guess.0[i] {
            exact_matches += 1;
        } else if colors & (1 << guess.0[i]) > 0 {
            inexact_matches += 1;
        }
    }
    assert!(exact_matches + inexact_matches <= FIELDS as u32);
    Evaluation {
        correct_color: inexact_matches,
        correct_color_and_position: exact_matches,
    }
}

struct DummyGuesser<const FIELDS: usize>;

impl<const FIELDS: usize> Solver<FIELDS> for DummyGuesser<FIELDS> {
    fn guess(&mut self, _history: &[Entry<FIELDS>]) -> Guess<FIELDS> {
        Guess([0; FIELDS])
    }
}

struct SimpleGuesser<const FIELDS: usize, const COLORS: u8>;

impl<const FIELDS: usize, const COLORS: u8> Solver<FIELDS> for SimpleGuesser<FIELDS, COLORS> {
    fn guess(&mut self, history: &[Entry<FIELDS>]) -> Guess<FIELDS> {
        let codes = self.generate_valid_codes(history);
        #[cfg(feature = "laura")]
        let iter = CodeIterator::<FIELDS, COLORS>::default();
        #[cfg(not(feature = "laura"))]
        let iter = GuessIterator::<FIELDS, COLORS>::default();

        let guess = iter
            .map(|mut guess| {
                const MAX_GAUSS: usize = Evaluation::<{ NUM_FIELDS as usize }>::MAX_GAUSS;

                let mut counts: [u32; MAX_GAUSS] = [0; MAX_GAUSS];
                for code in codes.iter() {
                    let result = evaluate(*code, guess);
                    counts[result.to_u32() as usize] += 1;
                }
                let max = counts.iter().max().unwrap();
                if *max == 1 {
                    guess = codes[0];
                }
                (guess, *max)
            })
            .min_by_key(|(_, max)| *max)
            .unwrap();

        guess.0
    }
}

impl<const FIELDS: usize, const COLORS: u8> SimpleGuesser<FIELDS, COLORS> {
    fn code_is_valid(&self, history: &[Entry<FIELDS>], current_guess: Guess<FIELDS>) -> bool {
        for entry in history {
            debug_assert!(
                entry.evaluation.correct_color + entry.evaluation.correct_color_and_position
                    <= FIELDS as u32,
                "The provided evaluation was not valid"
            );
            if !(evaluate(current_guess, entry.guess) == entry.evaluation) {
                return false;
            }
        }
        true
    }
    fn generate_valid_codes(&self, history: &[Entry<FIELDS>]) -> Vec<Guess<FIELDS>> {
        let mut valid_codes = Vec::new();
        for code in CodeIterator::<FIELDS, COLORS>::default() {
            if self.code_is_valid(history, code) {
                valid_codes.push(code);
            }
        }
        valid_codes
    }
}

fn interactive() {
    let mut guesser: SimpleGuesser<{ NUM_FIELDS as usize }, { NUM_COLORS }> = SimpleGuesser;
    let mut history = vec![];
    loop {
        let next_guess = guesser.guess(history.as_slice());
        println!("\nI'm guessing: {}", next_guess);

        print!("input correct colors (white):");
        std::io::stdout().flush().unwrap();
        let mut colors = String::new();
        std::io::stdin().read_line(&mut colors).unwrap();
        let colors: u32 = colors.trim().parse().unwrap();

        print!("input exact_matches (red):");
        std::io::stdout().flush().unwrap();
        let mut exact_matches = String::new();
        std::io::stdin().read_line(&mut exact_matches).unwrap();
        let exact_matches: u32 = exact_matches.trim().parse().unwrap();

        history.push(Entry {
            guess: next_guess,
            evaluation: Evaluation {
                correct_color: colors,
                correct_color_and_position: exact_matches,
            },
        });
    }
}

fn main() {
    interactive();
    /*
    let mut guesser: SimpleGuesser<4, NUM_COLORS> = SimpleGuesser;
    let mut history = vec![];
    let code = Guess([3, 9, 15, 4]);
    /*let first_guess = Guess([3, 9, 6, 6]);
    let result = evaluate(code, first_guess);
    history.push(Entry {
        guess: first_guess,
        evaluation: result,
    });*/
    for _ in 0..6 {
        let next_guess = guesser.guess(history.as_slice());
        history.push(Entry {
            guess: next_guess,
            evaluation: evaluate(code, next_guess),
        });
        println!("I'm guessing: {:?}", next_guess);
    }*/
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn dummy_guesser() {
        let guess = DummyGuesser.guess(&[]);
        assert_eq!(guess.0, [0, 0, 0, 0]);
    }

    #[test]
    fn evaluate_guess() {
        let code = Guess([1, 2, 3, 4]);
        let guess = Guess([1, 3, 3, 5]);
        let result = evaluate(code, guess);
        assert_eq!(
            result,
            Evaluation {
                correct_color: 1,
                correct_color_and_position: 2
            }
        );
    }

    #[test]
    fn evaluate_guess_six_element_guess() {
        let code = Guess([1, 2, 3, 4, 6, 7]);
        let guess = Guess([1, 3, 6, 6, 6, 5]);
        let result = evaluate(code, guess);
        assert_eq!(
            result,
            Evaluation {
                correct_color: 3,
                correct_color_and_position: 2
            }
        );
    }

    #[test]
    fn generate_guess_iterator() {
        let mut iter = GuessIterator::<3, 4>::default();
        assert_eq!(iter.next(), Some(Guess([0, 0, 0])));
        assert_eq!(iter.next(), Some(Guess([1, 0, 0])));
        assert_eq!(iter.next(), Some(Guess([2, 0, 0])));
        assert_eq!(iter.next(), Some(Guess([3, 0, 0])));
        assert_eq!(iter.next(), Some(Guess([0, 1, 0])));
        assert_eq!(iter.next(), Some(Guess([1, 1, 0])));
        assert_eq!(iter.next(), Some(Guess([2, 1, 0])));
        assert_eq!(iter.next(), Some(Guess([3, 1, 0])));
        let mut iter = iter.skip(55);
        assert_eq!(iter.next(), Some(Guess([3, 3, 3])));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_color_bitmask() {
        assert!(NUM_COLORS as usize <= std::mem::size_of::<ColorBitmask>() * 8);
    }

    #[test]
    fn test_color_fields() {
        assert!(NUM_COLORS as u32 >= NUM_FIELDS);
    }

    #[test]
    fn generate_code_iterator() {
        let mut iter = CodeIterator::<3, 4>::default();
        assert_eq!(iter.next(), Some(Guess([2, 1, 0])));
        assert_eq!(iter.next(), Some(Guess([3, 1, 0])));
        assert_eq!(iter.next(), Some(Guess([1, 2, 0])));
        assert_eq!(iter.next(), Some(Guess([3, 2, 0])));
        assert_eq!(iter.next(), Some(Guess([1, 3, 0])));
        assert_eq!(iter.next(), Some(Guess([2, 3, 0])));
        assert_eq!(iter.next(), Some(Guess([2, 0, 1])));
    }

    #[test]
    fn evaluation_to_u32_one_zero() {
        let evaluation: Evaluation<3> = Evaluation {
            correct_color: 1,
            correct_color_and_position: 0,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 4);
    }
    #[test]
    fn evaluation_to_u32_zero_zero() {
        let evaluation: Evaluation<3> = Evaluation {
            correct_color: 0,
            correct_color_and_position: 0,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 0);
    }
    #[test]
    fn evaluation_to_u32_zero_one() {
        let evaluation: Evaluation<3> = Evaluation {
            correct_color: 0,
            correct_color_and_position: 1,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 1);
    }
    #[test]
    fn evaluation_to_u32_one_two() {
        let evaluation: Evaluation<3> = Evaluation {
            correct_color: 1,
            correct_color_and_position: 2,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 6);
    }
    #[test]
    fn evaluation_to_u32_two_one() {
        let evaluation: Evaluation<3> = Evaluation {
            correct_color: 2,
            correct_color_and_position: 1,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 8);
    }
    #[test]
    fn evaluation_to_u32_three_zero() {
        let evaluation: Evaluation<3> = Evaluation {
            correct_color: 3,
            correct_color_and_position: 0,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 9);
    }
    #[test]
    fn evaluation_to_u32_four_fields_one_three() {
        let evaluation: Evaluation<4> = Evaluation {
            correct_color: 1,
            correct_color_and_position: 3,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 8);
    }

    extern crate test;
    use test::{black_box, Bencher};
    #[bench]
    fn guess_with_emty_history(b: &mut Bencher) {
        let mut guesser: SimpleGuesser<4, 8> = SimpleGuesser;
        let history = vec![];
        b.iter(|| black_box(guesser.guess(history.as_slice())));
    }
}
