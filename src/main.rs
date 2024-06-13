use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::stream::StreamExt;
use futures_util::SinkExt;

use reqwest::Client;
use tokio;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}


#[derive(Deserialize, Debug)]
struct OpenAIChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize, Debug)]
struct ChatChoice {
    message: ChatMessageContent,
}

#[derive(Deserialize, Debug)]
struct ChatMessageContent {
    content: String,
}

async fn request_openai(prompt: &str, api_key: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {

    let client = Client::new();
    let request = OpenAIChatRequest {
        model: "gpt-3.5-turbo-0125".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
        max_tokens: 50,
    };

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await?;

    let response_text = response.text().await?;
    println!("Raw response: {}", response_text); // Debug: print the raw response

    let openai_response: OpenAIChatResponse = serde_json::from_str(&response_text)?;
    println!("Parsed response: {:?}", openai_response); // Debug: print the parsed response

    Ok(openai_response.choices.first().map_or_else(|| "No response".to_string(), |choice| choice.message.content.clone()))

}

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:7746";
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");

    println!("Listening on: {}", addr);
    
    let api_key = "sk-proj-6wrWCt8PxMTirlQo3VnlT3BlbkFJ3OOnxw4falfMYCaWn9ur";

    while let Ok((stream, _)) = listener.accept().await {


        let api_key = api_key.to_string(); // Clone the API key for each connection

        tokio::spawn(async move {
            let ws_stream = accept_async(stream).await.expect("Failed to accept");
            println!("New WebSocket connection");

            let (mut write, mut read) = ws_stream.split();

            while let Some(Ok(message)) = read.next().await {
                if let Message::Text(text) = message {
                    println!("Received message: {}", text);

                    match request_openai(&text, &api_key).await {
                        Ok(response) => {
                            write.send(Message::Text(format!("OpenAI: {}", response))).await.expect("Failed to send message");
                        }
                        Err(e) => {
                            eprintln!("Failed to request OpenAI API: {}", e);
                            write.send(Message::Text("Error processing request".to_string())).await.expect("Failed to send message");
                        }
                    }
                }
            }
        });
    }
}

