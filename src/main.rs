mod quiz;

use std::{fs::File, sync::Arc};

use chatgpt::{client::ChatGPT, config::ChatGPTEngine};
use dotenv::dotenv;
use log::debug;
use quiz::{ai_helper::QuizHelper, parts::PartsSentences};
use teloxide::{
    dispatching::dialogue::{serializer::Json, ErasedStorage, SqliteStorage, Storage},
    prelude::*,
    types::{ChatAction, KeyboardButton, KeyboardMarkup, ParseMode, True},
};

type QuizDialogue = Dialogue<State, ErasedStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub enum State {
    #[default]
    Start,
    ReceiveFullName,
    RecieveGameChoice,
    StressedWordsQuizRecieveAmountOfQuestions,
    StressedWordsQuiz {
        quiz: quiz::Quiz,
        question_number: usize,
        score: usize,
    },
    PartsOfSpeechRecieveAmountOfQuestions,
    PartsOfSpeechQuiz {
        quiz: quiz::Quiz,
        question_number: usize,
        score: usize,
    },
}

type UserInfoStorage = std::sync::Arc<ErasedStorage<State>>;

#[tokio::main]
async fn main() {
    dotenv().expect("Failed to load .env file");
    let CHATGPT_API_KEY = std::env::var("CHATGPT_API_KEY").expect("CHATGPT_API_KEY is not set");

    pretty_env_logger::init();
    log::info!("Starting dialogue bot...");

    let bot = Bot::from_env();

    println!("Establishing connection to the database...");
    let storage: UserInfoStorage = SqliteStorage::open("db.sqlite", Json)
        .await
        .unwrap()
        .erase();
    println!("Connection established");

    // Load the dictionary of stressed words
    println!("Loading the dictionary of stressed words");
    let stressed_words_dictionary = Arc::new(quiz::stress::StressWords::new(
        File::open("stress.txt").expect("Failed to open file 'stress.txt'"),
    ));
    println!("Dictionary loaded");

    // Load the conllu file w/ the Ukrainian treebank
    // TODO? Implement a way to use the Ukrainian treebank to generate questions

    println!("Loading the conllu file");
    let conllu_file = File::open("uk_iu-ud-dev.conllu").expect("Failed to open conllu file");
    let conllu_doc = PartsSentences::new(conllu_file);
    let conllu_doc = Arc::new(conllu_doc);

    println!("Conllu file loaded");

    let gpt = {
        let mut gpt = ChatGPT::new(CHATGPT_API_KEY).expect("Unable to connect with ChatGPT");

        gpt.config.engine = ChatGPTEngine::Gpt35Turbo;
        gpt.config.timeout = std::time::Duration::from_secs(15);

        gpt
    };

    let quiz_helper = Arc::new(QuizHelper::new(
        gpt,
        quiz::ai_helper::Personality::Shevchenko,
    ));
    let quiz_helper_for_parts = quiz_helper.clone();

    Dispatcher::builder(
        bot,
        Update::filter_message()
            .enter_dialogue::<Message, ErasedStorage<State>, State>()
            .branch(dptree::case![State::Start].endpoint(start))
            .branch(dptree::case![State::ReceiveFullName].endpoint(receive_full_name))
            .branch(dptree::case![State::RecieveGameChoice].endpoint(receive_game_choice))
            .branch(
                dptree::case![State::StressedWordsQuizRecieveAmountOfQuestions].endpoint(
                    move |bot: Bot, dialogue: QuizDialogue, msg: Message| {
                        receive_amount_of_questions(
                            stressed_words_dictionary.clone(),
                            bot,
                            dialogue,
                            msg,
                        )
                    },
                ),
            )
            .branch(
                dptree::case![State::StressedWordsQuiz {
                    quiz,
                    question_number,
                    score
                }]
                .endpoint(
                    move |bot: Bot,
                          dialogue: QuizDialogue,
                          (quiz, question_number, score): (quiz::Quiz, usize, usize),
                          msg: Message| {
                        stressed_quiz(
                            quiz_helper.clone(),
                            bot,
                            dialogue,
                            (quiz.clone(), question_number, score),
                            msg,
                        )
                    },
                ),
            )
            .branch(
                dptree::case![State::PartsOfSpeechRecieveAmountOfQuestions].endpoint(
                    move |bot: Bot, dialogue: QuizDialogue, msg: Message| {
                        receive_amount_of_questions_parts_of_speech(
                            conllu_doc.clone(),
                            bot,
                            dialogue,
                            msg,
                        )
                    },
                ),
            )
            .branch(
                dptree::case![State::PartsOfSpeechQuiz {
                    quiz,
                    question_number,
                    score
                }]
                .endpoint(
                    move |bot: Bot,
                          dialogue: QuizDialogue,
                          (quiz, question_number, score): (quiz::Quiz, usize, usize),
                          msg: Message| {
                        parts_of_speech_quiz(
                            quiz_helper_for_parts.clone(),
                            bot,
                            dialogue,
                            (quiz.clone(), question_number, score),
                            msg,
                        )
                    },
                ),
            ),
    )
    .dependencies(dptree::deps![storage])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
}

const GREETING_TEXT: &str = "Привіт! Я -- морфологічний бот. Я допоможу тобі вивчити українську мову! Давай познайомимося! Як тебе звати?";
async fn start(bot: Bot, dialogue: QuizDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, GREETING_TEXT).await?;

    dialogue.update(State::ReceiveFullName).await?;
    Ok(())
}

const STRESSED_WORDS_GAME: &str = "Почати тест на наголос";
const PARTS_OF_SPEECH_GAME: &str = "Почати тест на частини мови";
async fn receive_full_name(bot: Bot, dialogue: QuizDialogue, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(full_name) => {
            bot.send_message(
                msg.chat.id,
                format!("Приємно познайомитися, {}!", full_name),
            )
            .await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Будь ласка, введіть своє ім'я (текстом)")
                .await?;
            return Ok(());
        }
    }

    let keyboard = KeyboardMarkup::new(vec![vec![
        KeyboardButton::new(STRESSED_WORDS_GAME),
        KeyboardButton::new(PARTS_OF_SPEECH_GAME),
    ]]);
    bot.send_message(msg.chat.id, "Що б ти хотів зробити?")
        .reply_markup(keyboard)
        .await?;

    dialogue.update(State::RecieveGameChoice).await?;
    return Ok(());
}

async fn receive_game_choice(bot: Bot, dialogue: QuizDialogue, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(STRESSED_WORDS_GAME) => {
            let keyboard = KeyboardMarkup::new(vec![
                vec![KeyboardButton::new("5")],
                vec![KeyboardButton::new("10")],
                vec![KeyboardButton::new("15")],
            ]);
            bot.send_message(msg.chat.id, "Обери кількість питань")
                .reply_markup(keyboard)
                .await?;
            dialogue
                .update(State::StressedWordsQuizRecieveAmountOfQuestions)
                .await?;
            return Ok(());
        }
        // TODO: Implement the parts of speech game
        Some(PARTS_OF_SPEECH_GAME) => {
            let keyboard = KeyboardMarkup::new(vec![
                vec![KeyboardButton::new("5")],
                vec![KeyboardButton::new("10")],
                vec![KeyboardButton::new("15")],
            ]);
            bot.send_message(msg.chat.id, "Обери кількість питань")
                .reply_markup(keyboard)
                .await?;
            dialogue
                .update(State::PartsOfSpeechRecieveAmountOfQuestions)
                .await?;
            return Ok(());
        }
        _ => {
            bot.send_message(msg.chat.id, "Будь ласка, виберіть один з варіантів")
                .await?;
            return Ok(());
        }
    }
}

async fn receive_amount_of_questions(
    dictionary: Arc<quiz::stress::StressWords>,
    bot: Bot,
    dialogue: QuizDialogue,
    msg: Message,
) -> HandlerResult {
    if let None = msg.text() {
        bot.send_message(msg.chat.id, "Будь ласка, введіть число")
            .await?;
        return Ok(());
    }
    if let Err(_) = msg.text().unwrap().parse::<usize>() {
        bot.send_message(msg.chat.id, "Будь ласка, введіть число")
            .await?;
        return Ok(());
    }

    // It is safe to unwrap here because we've already checked that the input is a number
    let amount: usize = msg.text().unwrap().parse().unwrap();
    if amount == 0 {
        bot.send_message(msg.chat.id, "Кількість питань не може бути 0")
            .await?;
        return Ok(());
    }

    let quiz = quiz::Quiz::new(
        (0..amount)
            .map(|_| dictionary.get_random_word().generate_question())
            .collect(),
    );

    bot.send_message(msg.chat.id, "Чудово! Почнемо тест!")
        .reply_markup(KeyboardMarkup::new(vec![vec![KeyboardButton::new("Вйо!")]]))
        .await?;

    dialogue
        .update(State::StressedWordsQuiz {
            quiz,
            question_number: 0,
            score: 0,
        })
        .await?;
    Ok(())
}

async fn stressed_quiz(
    ai_helper: Arc<QuizHelper>,
    bot: Bot,
    dialogue: QuizDialogue,
    (quiz, question_number, score): (quiz::Quiz, usize, usize),
    msg: Message,
) -> HandlerResult {
    let mut current_score = score;
    if question_number != 0 {
        let answer = msg.text().unwrap();
        let question = &quiz.questions[question_number - 1];
        let correct_answer = question.answers.iter().find(|a| a.is_correct).unwrap();
        if answer == correct_answer.text {
            bot.send_message(msg.chat.id, "Правильно!").await?;
            current_score += 1;
        } else {
            let ai_reply = ai_helper
                .generate_reply_to_wrong_stress_answer(question.clone())
                .await?;

            bot.send_message(msg.chat.id, ai_reply)
                .parse_mode(ParseMode::Html)
                .await?;
        }
    }

    if question_number >= quiz.questions.len() {
        let keyboard = KeyboardMarkup::new(vec![vec![
            KeyboardButton::new(STRESSED_WORDS_GAME),
            KeyboardButton::new(PARTS_OF_SPEECH_GAME),
        ]]);
        let quiz_score = format!(
            "Квіз закінчився! Ти відповів правильно на {} з {} питань\nЩо б ти хотів зробити далі?",
            current_score,
            quiz.questions.len()
        );
        bot.send_message(msg.chat.id, quiz_score.as_str())
            .reply_markup(keyboard)
            .await?;

        dialogue.update(State::RecieveGameChoice).await?;
        return Ok(());
    }

    let question = &quiz.questions[question_number];

    let text_question_from_answers = question
        .answers
        .iter()
        .map(|a| a.text.clone())
        .collect::<Vec<_>>()
        .join(" чи ");

    let ai_example = ai_helper
        .generate_example_for_stress_question(question.clone())
        .await?;

    let question_text = format!(
        "Питання №{}: \n{}?\n\nПриклад:\n{}",
        question_number + 1,
        text_question_from_answers,
        ai_example
    );

    let mut answers = Vec::new();
    for answer in &question.answers {
        answers.push(KeyboardButton::new(answer.text.clone()));
    }

    bot
        .send_message(msg.chat.id, question_text)
        .parse_mode(ParseMode::Html)
        .reply_markup(KeyboardMarkup::new(vec![answers]))
        .await?;

    dialogue
        .update(State::StressedWordsQuiz {
            quiz,
            question_number: question_number + 1,
            score: current_score,
        })
        .await?;
    Ok(())
}

async fn receive_amount_of_questions_parts_of_speech(
    colllu_doc: Arc<PartsSentences>,
    bot: Bot,
    dialogue: QuizDialogue,
    msg: Message,
) -> HandlerResult {
    if let None = msg.text() {
        bot.send_message(msg.chat.id, "Будь ласка, введіть число")
            .await?;
        return Ok(());
    }
    if let Err(_) = msg.text().unwrap().parse::<usize>() {
        bot.send_message(msg.chat.id, "Будь ласка, введіть число")
            .await?;
        return Ok(());
    }

    // It is safe to unwrap here because we've already checked that the input is a number
    let amount: usize = msg.text().unwrap().parse().unwrap();
    if amount == 0 {
        bot.send_message(msg.chat.id, "Кількість питань не може бути 0")
            .await?;
        return Ok(());
    }

    let quiz = quiz::Quiz::new(
        (0..amount)
            .map(|_| {
                let random_sentence = colllu_doc.get_random_sentence();
                random_sentence.generate_question()
            })
            .collect(),
    );

    bot.send_message(msg.chat.id, "Чудово! Почнемо тест!")
        .reply_markup(KeyboardMarkup::new(vec![vec![KeyboardButton::new("Вйо!")]]))
        .await?;

    dialogue
        .update(State::PartsOfSpeechQuiz {
            quiz,
            question_number: 0,
            score: 0,
        })
        .await?;
    Ok(())
}

async fn parts_of_speech_quiz(
    ai_helper: Arc<QuizHelper>,
    bot: Bot,
    dialogue: QuizDialogue,
    (quiz, question_number, score): (quiz::Quiz, usize, usize),
    msg: Message,
) -> HandlerResult {
    let mut current_score = score;
    if question_number != 0 {
        let answer = msg.text().unwrap();
        let question = &quiz.questions[question_number - 1];
        let correct_answer = question.answers.iter().find(|a| a.is_correct).unwrap();
        if answer == correct_answer.text {
            bot.send_message(msg.chat.id, "Правильно!").await?;
            current_score += 1;
        } else {
            // We don't really care about the result here, so we'll just ignore the error if this action is unsuccessful
            // But it adds to the user's experience if it works!
            let _ = bot.send_chat_action(msg.chat.id, ChatAction::Typing)
                .await;

            let ai_reply: String = ai_helper
                .generate_reply_to_wrong_parts_answer(question.clone())
                // If the AI fails to generate a reply, we'll just tell the user the correct answer
                // Sometimes it may happen due to timeout or other reasons
                .await.unwrap_or(format!("Правильна відповідь -- {} Будь уважнішим!", correct_answer.text));

            bot.send_message(msg.chat.id, format!("Неправильно!\n\n{}", ai_reply)).await?;
        }
    }

    if question_number >= quiz.questions.len() {
        let keyboard = KeyboardMarkup::new(vec![vec![
            KeyboardButton::new(STRESSED_WORDS_GAME),
            KeyboardButton::new(PARTS_OF_SPEECH_GAME),
        ]]);
        let quiz_score = format!(
            "Квіз закінчився! Ти відповів правильно на {} з {} питань\nЩо б ти хотів зробити далі?",
            current_score,
            quiz.questions.len()
        );
        bot.send_message(msg.chat.id, quiz_score.as_str())
            .reply_markup(keyboard)
            .await?;

        dialogue.update(State::RecieveGameChoice).await?;
        return Ok(());
    }

    let question = &quiz.questions[question_number];
    let answers = &question
        .answers
        .iter()
        .map(|a| a.text.clone())
        .collect::<Vec<_>>();

    bot.send_message(msg.chat.id, question.clone().text)
        .parse_mode(ParseMode::Html)
        .reply_markup(KeyboardMarkup::new(
            answers
                .iter()
                .map(|a| vec![KeyboardButton::new(a.clone())])
                .collect::<Vec<_>>(),
        ))
        .await?;

    dialogue
        .update(State::PartsOfSpeechQuiz {
            quiz,
            question_number: question_number + 1,
            score: current_score,
        })
        .await?;
    Ok(())
}
