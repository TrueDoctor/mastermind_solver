pub const NUM_COLORS: u8 = 8;
pub const NUM_FIELDS: u32 = 4;
pub type ColorBitmask = u8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Guess<const FIELDS: usize>([u8; FIELDS]);

impl<const FIELDS: usize> Default for Guess<FIELDS> {
    fn default() -> Self {
        Self([0; FIELDS])
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

impl<const FIELDS_PLUS_ONE: usize> Evaluation<FIELDS_PLUS_ONE> {
    const MAX_GAUSS: usize = (FIELDS_PLUS_ONE as usize + 1) * FIELDS_PLUS_ONE as usize / 2;
    fn lut() -> [usize; FIELDS_PLUS_ONE] {
        std::array::from_fn(|i| {
            println!("{}", ((i + 2) * (i + 1) / 2) - 1);
            (i + 2) * (i + 1) / 2 - 1
        })
    }
    pub fn to_u32(&self) -> u32 {
        Evaluation::<FIELDS_PLUS_ONE>::MAX_GAUSS as u32
            - 1
            - Evaluation::<FIELDS_PLUS_ONE>::lut()
                [FIELDS_PLUS_ONE - 1 - self.correct_color as usize] as u32
            + self.correct_color_and_position
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
    let mut colors = 0u8;

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

struct SimpleGuesser<const FIELDS: usize>;

impl<const FIELDS: usize> Solver<FIELDS> for SimpleGuesser<FIELDS> {
    fn guess(&mut self, history: &[Entry<FIELDS>]) -> Guess<FIELDS> {
        for guess in GuessIterator::<FIELDS, NUM_COLORS>::default() {
            for code in self.genrate_valid_codes(history) {
                let result = evaluate(code, guess);
                result.to_u32();
            }
        }

        Guess([0; FIELDS])
    }
}

impl<const FIELDS: usize> SimpleGuesser<FIELDS> {
    fn code_is_valid(&self, history: &[Entry<FIELDS>], current_guess: Guess<FIELDS>) -> bool {
        for entry in history {
            if !(evaluate(current_guess, entry.guess) == entry.evaluation) {
                return false;
            }
        }
        true
    }
    fn genrate_valid_codes(&self, history: &[Entry<FIELDS>]) -> Vec<Guess<FIELDS>> {
        let mut valid_codes = Vec::new();
        for code in CodeIterator::<FIELDS, NUM_COLORS>::default() {
            if self.code_is_valid(history, code) {
                valid_codes.push(code);
            }
        }
        valid_codes
    }
}

fn main() {
    let guess = DummyGuesser.guess(&[]);
    assert_eq!(guess.0, [0, 0, 0, 0]);
    let code = Guess([1, 2, 3, 4]);
    let guess = Guess([1, 3, 3, 5]);
    let result = evaluate(code, guess);

    println!("Hello, world! {:?}", result);

    for guess in CodeIterator::<3, 4>::default() {
        println!("{guess:?}");
    }
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
        let evaluation: Evaluation<4> = Evaluation {
            correct_color: 1,
            correct_color_and_position: 0,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 4);
    }
    #[test]
    fn evaluation_to_u32_zero_zero() {
        let evaluation: Evaluation<4> = Evaluation {
            correct_color: 0,
            correct_color_and_position: 0,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 0);
    }
    #[test]
    fn evaluation_to_u32_zero_one() {
        let evaluation: Evaluation<4> = Evaluation {
            correct_color: 0,
            correct_color_and_position: 1,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 1);
    }
    #[test]
    fn evaluation_to_u32_one_two() {
        let evaluation: Evaluation<4> = Evaluation {
            correct_color: 1,
            correct_color_and_position: 2,
        };
        let result = evaluation.to_u32();
        assert_eq!(result, 6);
    }
}
