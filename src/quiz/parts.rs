use std::fs::File;

use crate::quiz;
use rand::prelude::*;
use rand::Rng;

pub struct PartsSentences {
    pub sentenses: Vec<PartsSentence>,
}

impl PartsSentences {
    pub fn new(file: File) -> Self {
        let conllu_doc: Vec<PartsSentence> = rs_conllu::parse_file(file)
            .filter(|sentence| sentence.is_ok())
            // We can unwrap safely here because we've already filtered out the errors
            .map(|sentence| sentence.unwrap())
            .map(|sentence| PartsSentence::new(sentence))
            .collect();
        Self {
            sentenses: conllu_doc,
        }
    }
    pub fn get_random_sentence(&self) -> &PartsSentence {
        let rand = rand::thread_rng().gen_range(0..self.sentenses.len());
        let rand_sentence = self.sentenses.get(rand).unwrap();
        return rand_sentence;
    }
}

pub struct PartsSentence {
    pub sentence: rs_conllu::Sentence,
}

impl PartsSentence {
    pub fn new(sentence: rs_conllu::Sentence) -> Self {
        Self { sentence }
    }
    pub fn generate_question(&self) -> quiz::Question {
        generate_question_out_of_sentence(&self.sentence)
    }
}
fn generate_question_out_of_sentence(sentence: &rs_conllu::Sentence) -> quiz::Question {
    let words_to_be_asked_about = sentence
        .tokens
        .iter()
        .filter(|t| t.upos != Some(rs_conllu::UPOS::PUNCT))
        .collect::<Vec<_>>();
    let random_word = words_to_be_asked_about
        .choose(&mut rand::thread_rng())
        .unwrap();
    let text_sentence = sentence
        .meta
        .iter()
        .find(|m| m.starts_with("text = "))
        .map(|m| m.replace("text = ", ""))
        .expect("Original 'text' metadata field not found on the sentence");

    let correct_answer = match random_word.upos {
        Some(rs_conllu::UPOS::ADJ) => "прикметник",
        Some(rs_conllu::UPOS::ADV) => "прислівник",
        Some(rs_conllu::UPOS::INTJ) => "вигук",
        Some(rs_conllu::UPOS::NOUN) => "іменник",
        Some(rs_conllu::UPOS::PROPN) => "власний іменник",
        Some(rs_conllu::UPOS::VERB) => "дієслово",

        Some(rs_conllu::UPOS::PRON) => "займенник",
        Some(rs_conllu::UPOS::ADP) => "прийменник",
        Some(rs_conllu::UPOS::CCONJ) => "сполучник",
        Some(rs_conllu::UPOS::SCONJ) => "підрядний сполучник",
        Some(rs_conllu::UPOS::AUX) => "допоміжне дієслово",
        Some(rs_conllu::UPOS::DET) => "детермінатив",
        Some(rs_conllu::UPOS::NUM) => "числівник",
        Some(rs_conllu::UPOS::PART) => "частка",

        Some(rs_conllu::UPOS::X) => "інше",
        Some(rs_conllu::UPOS::SYM) => "символ",
        Some(rs_conllu::UPOS::PUNCT) => "пунктуація",

        None => "інше",
    };
    let possible_answers = vec![
        "прикметник",
        "прислівник",
        "вигук",
        "іменник",
        "власний іменник",
        "дієслово",
        "займенник",
        "прийменник",
        "сполучник",
        "підрядний сполучник",
        "допоміжне дієслово",
        "детермінатив",
        "числівник",
        "частка",
       
        // I filter out those from possible words to be asked about
        // So I don't need to include them here
        // And they presense is a dead giveaway for the correct answer
        // "інше",
        // "символ",
        // "пунктуація",
    ];
    let incorrect_answer = possible_answers
        .iter()
        .filter(|a| a != &&correct_answer)
        .choose(&mut rand::thread_rng())
        .unwrap();

    let answers = {
        let mut shuffled_answers = vec![
            quiz::Answer::new(correct_answer.to_string(), true),
            quiz::Answer::new(incorrect_answer.to_string(), false),
        ];
        shuffled_answers.shuffle(&mut rand::thread_rng());
        // returns
        shuffled_answers
    };

    let text_sentence = text_sentence
        .split(" ")
        .map(|word| {
            let no_sep = word
                .chars()
                .filter(|c| c.is_alphanumeric())
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join("");
            if no_sep == random_word.form {
                return format!("<b><u>{}</u></b>", word);
            }
            return word.to_string();
        })
        .collect::<Vec<String>>()
        .join(" ");

    let question_text = format!(
        "У реченні:\n\"{}\"\n\nЯкою частиною мови є підкреслене слово \"{}\"?",
        text_sentence, random_word.form
    );
    return quiz::Question::new(question_text, answers);
}
