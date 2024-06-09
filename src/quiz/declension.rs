use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use serde_json::Value;

use crate::quiz;

pub struct Declension {
    pub noun_words: Vec<Noun>,
}

impl Declension {
    pub fn new(file: File) -> Self {
        let data: Vec<JsonWord> = serde_json::from_reader(file).expect("Unable to parse");
        let words: Vec<Noun> = data.iter()
        .filter(|x| x.pos == "noun")
        .map(|x| x.to_noun())
        .filter(|x| x.is_some())
        .map(|x| x.unwrap())
        .collect();

        return Self { noun_words: words };
    }

    pub fn get_random_noun(&self) -> &Noun {
        let rand = rand::thread_rng().gen_range(0..self.noun_words.len());
        let rand_word = self.noun_words.get(rand).unwrap();
        return rand_word;
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct JsonWord {
    pub word: String,
    pub pos: String,
    // pub defs: Vec<String>,
    // pub freq: Option<u32>,
    // pub info: String,
    pub forms: Value,
    // pub index: u32,
}
impl JsonWord {
    pub fn to_noun(&self) -> Option<Noun> {
        if self.pos != "noun" {
            // Maybe return an error instead?
            return None;
        }
        let mut noun_forms: Vec<NounForm> = Vec::new();
        let forms = self.forms.as_object().unwrap();
        for (case_plural, forms) in forms {
            // "forms" is an array of strings, always
            let splitted = case_plural.split_once(" ");
            if splitted.is_none() {
                continue;
            }
            let (case, plurality) = splitted.unwrap();

            let case = match case {
                "nom" => NounCase::Nominative, // називний
                "gen" => NounCase::Genitive, // родовий
                "dat" => NounCase::Dative, // давальний
                "acc" => NounCase::Accusative, // знахідний
                "ins" => NounCase::Instrumental, // орудний
                "loc" => NounCase::Locative, // місцевий
                "voc" => NounCase::Vocative, // кличний
                _ => continue
                // _ => panic!("Unknown case"),
            };
            // 'np' == noun plural
            // 'ns' == noun singular
            let is_plural = match plurality {
                "np" => true,
                "ns" => false,
                _ => continue
                // _ => panic!("Unknown plurality"),
            };
            noun_forms.push(NounForm {
                word: forms[0].as_str().unwrap().to_string(),
                case,
                is_plural,
            });                        
        }
        return Some(Noun {
            word: self.word.clone(),
            forms: noun_forms,
        })
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Noun {
    pub word: String,
    pub forms: Vec<NounForm>,
}
pub enum GenerateQuestionError {
    NoNominativeForm,
    NoCorrectAnswer,
}

impl Noun {
pub fn generate_question_out_of_noun(&self) -> Result<quiz::Question, GenerateQuestionError> {
        // let default_nominative_form = &self.forms.iter()
        // .find(|f| f.case == NounCase::Nominative && f.is_plural == false)
        // // or find plural if there is no singular
        // .or_else(|| self.forms.iter()
        //     .find(|f| f.case == NounCase::Nominative && f.is_plural == true)
        // )
        // .ok_or(GenerateQuestionError::NoNominativeForm)?;
        let default_nominative_form = &self.word;
        
        let random_case = NounCase::get_random_by_case_exluding_nominative();
        let random_plurality = rand::thread_rng().gen_bool(0.5);
    
        let text: String = format!("Поставте іменник \"{}\" у {} відмінок ({}) {} ", 
            default_nominative_form, 
            random_case.to_ukrainian_string(),
            random_case.ukrainian_question(),
            if random_plurality { "множини" } else { "однини" },
        );
    
        let correct_answer = &self.forms.iter()
        .find(|f| f.case == random_case && f.is_plural == random_plurality)
        // e.g. there is no plural form, but we need it
        .or_else(|| self.forms.iter().find(|f| f.case == random_case && f.is_plural != random_plurality))
        .ok_or(GenerateQuestionError::NoCorrectAnswer)?;

        let possible_non_correct_answers = &self.forms.iter()
        // Filter-out generic form
        .filter(|f| f.case != NounCase::Nominative)
        // Filter our correct answer too
        .filter(|f| f.case != correct_answer.case && f.is_plural == correct_answer.is_plural)
        .map(|f| f.word.clone())
        // Removes duplicates
        .collect::<HashSet<_>>().into_iter()
        .collect::<Vec<String>>();
    
        let answers = {
            let mut shuffled_answers = possible_non_correct_answers.clone().into_iter()
            .map(|a| quiz::Answer::new(a, false))
            .collect::<Vec<quiz::Answer>>();
    
            shuffled_answers.push(quiz::Answer::new(correct_answer.word.clone(), true));
            shuffled_answers.shuffle(&mut rand::thread_rng());
            // returns
            shuffled_answers
        };    
        return Ok(quiz::Question::new(text, answers));
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct NounForm {
    pub word: String,
    pub case: NounCase,
    pub is_plural: bool
}
impl NounForm {
    pub fn to_ukrainian_string(&self) -> String {
        let plurality = if self.is_plural { "множина" } else { "однина" };
        format!("{} ({} відмінок, {})", self.word, self.case.to_ukrainian_string(), plurality)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum NounCase {
    Nominative,
    Genitive,
    Dative,
    Accusative,
    Instrumental,
    Locative,
    Vocative,
}

impl NounCase {
    pub fn to_ukrainian_string(&self) -> &str {
        match self {
            NounCase::Nominative => "називний",
            NounCase::Genitive => "родовий",
            NounCase::Dative => "давальний",
            NounCase::Accusative => "знахідний",
            NounCase::Instrumental => "орудний",
            NounCase::Locative => "місцевий",
            NounCase::Vocative => "кличний",
        }
    }
    pub fn get_random_by_case() -> NounCase {
        let cases = vec![
            NounCase::Nominative,
            NounCase::Genitive,
            NounCase::Dative,
            NounCase::Accusative,
            NounCase::Instrumental,
            NounCase::Locative,
            NounCase::Vocative,
        ];
        let rand = rand::thread_rng().gen_range(0..cases.len());
        return cases.get(rand).unwrap().clone();
    }
    pub fn get_random_by_case_exluding_nominative() -> NounCase {
        let cases = vec![
            NounCase::Genitive,
            NounCase::Dative,
            NounCase::Accusative,
            NounCase::Instrumental,
            NounCase::Locative,
            NounCase::Vocative,
        ];
        let rand = rand::thread_rng().gen_range(0..cases.len());
        return cases.get(rand).unwrap().clone();
    }
    pub fn ukrainian_question(&self) -> &str {
        match self {
            NounCase::Nominative => "Хто? Що?",
            NounCase::Genitive => "Кого? Чого?",
            NounCase::Dative => "Кому? Чому?",
            NounCase::Accusative => "Кого? Що?",
            NounCase::Instrumental => "Ким? Чим?",
            NounCase::Locative => "На кому? На чому?",
            NounCase::Vocative => "Звертання до когось або чогось",            
        }
    }
}

impl Default for NounCase {
    fn default() -> Self {
        NounCase::Nominative
    }
}

