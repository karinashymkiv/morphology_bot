use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::quiz;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct StressWords {
    pub words: Vec<StressWord>,
}

impl StressWords {
    pub fn new(file: File) -> Self {
        let mut words: Vec<StressWord> = Vec::new();
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let word = line.expect("Failed to read line");
            words.push(StressWord::new(word));
        }

        return Self { words };
    }
    pub fn get_random_word(&self) -> &StressWord {
        let rand = rand::thread_rng().gen_range(0..self.words.len());
        let rand_word = self.words.get(rand).unwrap();
        // To avoid words with less than 2 vowels
        // since we need to have at least 2 vowels to stress one of them (duh!)
        let vowels_count = rand_word
            .word_without_stress_symbol
            .chars()
            .filter(|c| is_vowel(*c).unwrap())
            .count();
        if vowels_count < 2 {
            return self.get_random_word();
        }
        // To avoid phrases
        // TODO: filter out phrases our of the dictionary source file itself
        if rand_word.word_without_stress_symbol.contains(" ") {
            return self.get_random_word();
        }
        return rand_word;
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct StressWord {
    pub word_with_stress_symbol: String,
    pub word_without_stress_symbol: String,
}

// Голосні букви
const UKRAINIAN_VOWELS: [char; 10] = ['А', 'Е', 'Є', 'И', 'І', 'Ї', 'О', 'У', 'Ю', 'Я'];
fn is_vowel(c: char) -> Result<bool, ()> {
    let c = c.to_uppercase().next();
    if c.is_none() {
        return Err(());
    }
    return Ok(UKRAINIAN_VOWELS.contains(&c.unwrap()));
}
impl StressWord {
    fn new(word_with_stress_symbol: String) -> Self {
        let word_without_stress_symbol = Self::get_word_without_stress(&word_with_stress_symbol);
        return Self {
            word_with_stress_symbol,
            word_without_stress_symbol,
        };
    }

    fn get_word_without_stress(word: &str) -> String {
        return word.chars().filter(|c| c != &'\u{0301}').collect();
    }

    pub fn generate_question(&self) -> quiz::Question {
        let correct_stress = self.word_with_stress_symbol.clone();

        // Getting all of the indexes of the stress symbol's position (if there are multiple vowels in the word)
        // e.g. for "програмі́ст" it would be [7], sinse the 'і' is the only stressed vowel in the word
        // the actual \u{0301} symbol is at next, 8th position
        let stressed_idxs = correct_stress
            .chars()
            .enumerate()
            .filter(|(_, c)| *c == '\u{0301}')
            // So here we subtract 1 from the index to get the actual position of the stressed vowel
            .map(|(i, _)| i - 1)
            .collect::<Vec<_>>();

        // Generating a word with incorrect stress
        // Algorithm:
        // 1. Get all of the possible locations for the stress symbol (vowels positions)
        // 2. Remove the correct stress symbol's position from the list
        // 3. Choose one of the remaining positions randomly
        let incorrect_stress = {
            let possible_locations = self
                .word_without_stress_symbol
                .clone()
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if stressed_idxs.contains(&i) {
                        return None;
                    }

                    if let Ok(false) = is_vowel(c) {
                        return None;
                    }

                    return Some(i);
                })
                .filter(|x| x.is_some())
                .map(|x| x.unwrap())
                .collect::<Vec<_>>();

            let one_incorrect_stress = possible_locations.choose(&mut thread_rng()).unwrap();

            let mut incorrect_stress = self
                .word_without_stress_symbol
                .clone()
                .chars()
                .collect::<Vec<char>>();
            incorrect_stress.insert(*one_incorrect_stress + 1, '\u{0301}');
            incorrect_stress.iter().collect::<String>()
        };

        // We shuffle the answers so the correct one isn't always the first one
        let answers = {
            let mut shuffled_answers = vec![
                quiz::Answer::new(correct_stress, true),
                quiz::Answer::new(incorrect_stress, false),
            ];
            shuffled_answers.shuffle(&mut rand::thread_rng());
            // returns
            shuffled_answers
        };

        //
        let question = format!(
            "<b><i>{}</i></b> чи <b><i>{}</i></b> ?",
            answers[0].text, answers[1].text
        );

        return quiz::Question::new(question, answers);
    }
}
