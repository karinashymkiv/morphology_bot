pub mod ai_helper;
pub mod parts;
pub mod stress;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Quiz {
    pub questions: Vec<Question>,
    pub current_question: usize,
    pub score: u32,
}

impl Quiz {
    pub fn new(questions: Vec<Question>) -> Self {
        Self {
            questions,
            current_question: 0,
            score: 0,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Question {
    pub text: String,
    pub answers: Vec<Answer>,
}
impl Question {
    pub fn new(text: String, answers: Vec<Answer>) -> Self {
        Self { text, answers }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Answer {
    pub text: String,
    pub is_correct: bool,
}
impl Answer {
    pub fn new(text: String, is_correct: bool) -> Self {
        Self { text, is_correct }
    }
}
