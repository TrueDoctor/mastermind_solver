#![feature(test)]
use rayon::prelude::*;

use std::{cmp::Ordering, fmt::Display, io::Write};

pub const NUM_COLORS: u32 = 10;
pub const NUM_FIELDS: u32 = 6;
pub type ColorBitmask = u32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Guess<const FIELDS: usize>([u32; FIELDS]);

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
    fn iter<const NUM_COLORS: u32>(&self) -> GuessIterator<FIELDS, NUM_COLORS> {
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
    exact: u32,
}

#[inline]
pub const fn max_gauss(i: usize) -> usize {
    (i + 2) * (i + 1) / 2
}

impl<const FIELDS: usize> Evaluation<FIELDS> {
    const MAX_GAUSS: u32 = (FIELDS as u32 + 2) * (FIELDS + 1) as u32 / 2;
    #[inline]
    const fn lut_for_index(i: u32) -> u32 {
        (i + 2) * (i + 1) / 2
    }
    #[inline]
    pub fn to_u32(&self) -> u32 {
        Self::MAX_GAUSS as u32 + self.exact
            - Self::lut_for_index(FIELDS as u32 - self.correct_color)
    }
}

pub struct Entry<const FIELDS: usize> {
    guess: Guess<FIELDS>,
    evaluation: Evaluation<FIELDS>,
}

#[derive(Default)]
pub struct GuessIterator<const FIELDS: usize, const COLORS: u32> {
    current: Guess<FIELDS>,
    exhausted: bool,
}

impl<const FIELDS: usize, const COLORS: u32> Iterator for GuessIterator<FIELDS, COLORS> {
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
pub struct CodeIterator<const FIELDS: usize, const COLORS: u32> {
    current: Guess<FIELDS>,
}

impl<const FIELDS: usize, const COLORS: u32> Iterator for CodeIterator<FIELDS, COLORS> {
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
    fn guess(&mut self, history: &[Entry<FIELDS>]) -> (Guess<FIELDS>, f64);
}

//#[inline(never)]
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
        exact_matches += (code.0[i] == guess.0[i]) as u32;
        inexact_matches += (colors & (1 << guess.0[i]) > 0) as u32;
    }
    debug_assert!(inexact_matches <= FIELDS as u32);
    Evaluation {
        correct_color: inexact_matches - exact_matches,
        exact: exact_matches,
    }
}

struct DummyGuesser<const FIELDS: usize>;

impl<const FIELDS: usize> Solver<FIELDS> for DummyGuesser<FIELDS> {
    fn guess(&mut self, _history: &[Entry<FIELDS>]) -> (Guess<FIELDS>, f64) {
        (Guess([0; FIELDS]), 0.)
    }
}

struct SimpleGuesser<const FIELDS: usize, const COLORS: u32, const PARTITIONS: usize>;

impl<const FIELDS: usize, const COLORS: u32, const PARTITIONS: usize> Solver<FIELDS>
    for SimpleGuesser<FIELDS, COLORS, PARTITIONS>
{
    fn guess(&mut self, history: &[Entry<FIELDS>]) -> (Guess<FIELDS>, f64) {
        let codes = self.generate_valid_codes(history);
        #[cfg(feature = "laura")]
        let iter = CodeIterator::<FIELDS, COLORS>::default();
        #[cfg(not(feature = "laura"))]
        let iter = GuessIterator::<FIELDS, COLORS>::default();
        let guesses: Vec<_> = iter.collect();

        let guess = guesses
            .par_iter()
            .map(|guess| {
                let guess = *guess;
                let mut counts = [0; PARTITIONS];
                for code in codes.iter() {
                    let result = evaluate(*code, guess);
                    let index = result.to_u32() as usize;
                    counts[index] += 1;
                }
                let sum: u32 = counts.iter().sum();
                let mut information: f64 = counts
                    .iter()
                    .map(|x| *x as f64 / sum as f64)
                    .map(|x| -x * x.log2())
                    .map(|x| if x.is_finite() { x } else { 0. })
                    .sum();
                if counts[FIELDS as usize] == 1 && sum == 1 {
                    information += PARTITIONS as f64 - 1.;
                }
                /*if counts[FIELDS as usize] != 0 {
                    println!(
                        "guess: {guess} \t\t\t\t | {information:?}, {}",
                        counts[FIELDS]
                    );
                }*/
                (guess, information)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Greater))
            .unwrap();

        println!("avg: {:?}", guess.1);
        guess
    }
}

impl<const FIELDS: usize, const COLORS: u32, const PARTITIONS: usize>
    SimpleGuesser<FIELDS, COLORS, PARTITIONS>
{
    fn code_is_valid(&self, history: &[Entry<FIELDS>], current_guess: Guess<FIELDS>) -> bool {
        for entry in history {
            debug_assert!(
                entry.evaluation.correct_color + entry.evaluation.exact <= FIELDS as u32,
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
    let mut guesser: SimpleGuesser<
        { NUM_FIELDS as usize },
        { NUM_COLORS },
        { max_gauss(NUM_FIELDS as usize) },
    > = SimpleGuesser;
    let mut history = vec![];
    loop {
        let (next_guess, _score) = guesser.guess(history.as_slice());
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
                exact: exact_matches,
            },
        });
    }
}

fn main() {
    //interactive();

    let mut guesser: SimpleGuesser<
        { NUM_FIELDS as usize },
        NUM_COLORS,
        { max_gauss(NUM_FIELDS as usize) },
    > = SimpleGuesser;
    let mut history = vec![];
    let code = Guess([3, 2, 1, 0, 6, 5]);
    loop {
        let (next_guess, score) = guesser.guess(history.as_slice());
        history.push(Entry {
            guess: next_guess,
            evaluation: evaluate(code, next_guess),
        });
        println!("I'm guessing: [{}] ({} bit)", next_guess, score);
        if code == next_guess {
            break;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn dummy_guesser() {
        let guess = DummyGuesser.guess(&[]);
        assert_eq!(guess.0 .0, [0, 0, 0, 0]);
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
                exact: 2
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
                exact: 2
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
        assert!(NUM_COLORS >= NUM_FIELDS);
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
            exact: 0,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 4);
    }
    #[test]
    fn evaluation_to_u32_zero_zero() {
        let evaluation: Evaluation<3> = Evaluation {
            correct_color: 0,
            exact: 0,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 0);
    }
    #[test]
    fn evaluation_to_u32_zero_one() {
        let evaluation: Evaluation<3> = Evaluation {
            correct_color: 0,
            exact: 1,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 1);
    }
    #[test]
    fn evaluation_to_u32_one_two() {
        let evaluation: Evaluation<3> = Evaluation {
            correct_color: 1,
            exact: 2,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 6);
    }
    #[test]
    fn evaluation_to_u32_two_one() {
        let evaluation: Evaluation<3> = Evaluation {
            correct_color: 2,
            exact: 1,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 8);
    }
    #[test]
    fn evaluation_to_u32_three_zero() {
        let evaluation: Evaluation<3> = Evaluation {
            correct_color: 3,
            exact: 0,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 9);
    }
    #[test]
    fn evaluation_to_u32_four_fields_one_three() {
        let evaluation: Evaluation<4> = Evaluation {
            correct_color: 1,
            exact: 3,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 8);
    }

    extern crate test;
    use test::{black_box, Bencher};
    #[bench]
    fn guess_with_emty_history(b: &mut Bencher) {
        let mut guesser: SimpleGuesser<4, 8, { max_gauss(4) }> = SimpleGuesser;
        let history = vec![];
        black_box(guesser.guess(history.as_slice()));
        b.iter(|| black_box(guesser.guess(history.as_slice())));
    }
}
