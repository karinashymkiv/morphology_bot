use crate::quiz::Question;
use chatgpt::prelude::*;
use chatgpt::types::CompletionResponse;

pub struct QuizHelper {
    personality: Personality,
    chat_gpt: ChatGPT,
}
enum QuizHelperError {
    ChatGPTError(chatgpt::err::Error),
    NoWrongAnswerError,
    NoCorrectAnswerError,
}
impl QuizHelper {
    pub fn new(chat_gpt: ChatGPT, personality: Personality) -> Self {
        Self {
            personality,
            chat_gpt,
        }
    }

    pub async fn generate_example_for_stress_question(&self, question: Question) -> Result<String> {
        println!("Generating example for question: {:?}", question.text);
        let prompt = format!("Ти -- Чат-бот, який допомагає учням вивчати українську мову.
        Учню було задано питання про наголос у слові з двома варіянтами: \"{}\".
        Згенеруй речення де використовується це слово (не вказуючи наголос, звісно). До того ж напиши це речення так, наче ти -- {}", question.text, self.personality.get_personality());

        let response: CompletionResponse = self.chat_gpt.send_message(&prompt).await?;
        let content = response.message().clone().content;

        println!("Completion: {:?}", content);

        Ok(content)
    }

    pub async fn generate_reply_to_wrong_stress_answer(
        &self,
        question: Question,
    ) -> Result<String> {
        println!(
            "Generating reply to wrong answer for question: {:?}",
            question.text
        );
        let wrong_answer = question.answers.iter().find(|a| !a.is_correct).unwrap();
        let correct_answer = question.answers.iter().find(|a| a.is_correct).unwrap();

        let prompt = format!("Ти -- Чат-бот, який допомагає учням вивчати українську мову.
        Учень відповів неправильно на питання про наголос у слові; з двома варіянтами: \"{}\".
        Учень відповів {}, а правильна відповідь -- {}.
        Згенеруй відповідь, яка пояснює, чому правильний наголос саме на цьому слові. До того ж напиши це речення так, наче ти -- {}. Ліміт речення -- 100 символів.", question.text, wrong_answer.text, correct_answer.text, self.personality.get_personality());

        let response: CompletionResponse = self.chat_gpt.send_message(&prompt).await?;
        let content = response.message().clone().content;

        println!("Completion: {:?}", content);

        Ok(content)
    }

    pub async fn generate_reply_to_wrong_parts_answer(
        &self,
        question: Question,
    ) -> Result<String> {
        println!(
            "Generating reply to wrong answer for question: {:?}",
            question.text
        );
        let wrong_answer = question.answers.iter().find(|a| !a.is_correct)
        .ok_or(chatgpt::err::Error::BackendError { message: "No wrong answer found".to_string(), error_type: "QuizError".to_string() })?;

        let correct_answer = question.answers.iter().find(|a| a.is_correct)
        .ok_or(chatgpt::err::Error::BackendError { message: "No correct answer found".to_string(), error_type: "QuizError".to_string() })?;


        let prompt = format!("Ти -- Чат-бот, який допомагає учням вивчати українську мову.
        Учень вирішував задачу яка звучить так:
        {}
        Учень відповів {}, а правильна відповідь -- {}.
        Згенеруй відповідь, яка пояснює в чому була помилка, на яке питання відповідає правильна частина мови.
        До того ж напиши це речення так, наче ти -- {}. Ліміт 1-2 середніх абзаців.",
         question.text, wrong_answer.text, correct_answer.text, self.personality.get_personality());

        let response: CompletionResponse = self.chat_gpt.send_message(&prompt).await?;
        let content = response.message().clone().content;

        println!("Completion: {:?}", content);

        Ok(content)
    }
}

pub enum Personality {
    Shevchenko,
    Lesya,
    Franko,
}
impl Personality {
    pub fn get_personality(&self) -> String {
        match self {
            Personality::Shevchenko => "Тарас Шевченко",
            Personality::Lesya => "Леся Українка",
            Personality::Franko => "Іван Франко",
        }
        .to_string()
    }
}
