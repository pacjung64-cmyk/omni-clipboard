use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LensAction {
    Explain,
    TranslateThEn,
    FixCode,
    PolishTone,
    Custom(String),
}

impl LensAction {
    pub fn name(&self) -> &'static str {
        match self {
            LensAction::Explain => "Explain",
            LensAction::TranslateThEn => "Translate (TH/EN)",
            LensAction::FixCode => "Fix & Optimize Code",
            LensAction::PolishTone => "Polish Tone",
            LensAction::Custom(_) => "Custom Query",
        }
    }

    pub fn system_instructions(&self) -> &'static str {
        match self {
            LensAction::Explain => {
                "You are an expert concise assistant. Explain the following text simply and clearly in 1-3 bullet points or a short paragraph."
            }
            LensAction::TranslateThEn => {
                "You are an expert bilingual translator. If the text is in English, translate it to natural, fluent Thai. If the text is in Thai or any other language, translate it to English. Output only the translation without conversational filler."
            }
            LensAction::FixCode => {
                "You are a principal software engineer. Review the provided code snippet, fix any bugs, optimize performance and clean up syntax. Provide the clean corrected code block followed by a brief 1-2 sentence explanation of what was fixed."
            }
            LensAction::PolishTone => {
                "You are an expert editor. Rewrite the following text to sound professional, persuasive, and clear, while retaining the original core meaning. Output only the polished version."
            }
            LensAction::Custom(_) => {
                "You are an intelligent, helpful AI assistant. Answer the user prompt directly, concisely, and accurately based on the provided context."
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiProvider {
    Gemini { api_key: String, model: String },
    Groq { api_key: String, model: String },
    Ollama { endpoint: String, model: String },
    OpenAiCompatible { base_url: String, api_key: String, model: String },
}

impl Default for AiProvider {
    fn default() -> Self {
        AiProvider::Ollama {
            endpoint: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
        }
    }
}

pub trait AiLensService: Send + Sync {
    fn process(&self, action: &LensAction, text: &str) -> Result<String, String>;
}

pub struct LensClient {
    pub provider: AiProvider,
    http_client: reqwest::Client,
}

impl LensClient {
    pub fn new(provider: AiProvider) -> Self {
        Self {
            provider,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn process_async(&self, action: &LensAction, input_text: &str) -> Result<String, String> {
        let system_prompt = action.system_instructions();
        let user_prompt = match action {
            LensAction::Custom(custom) => format!("Context:\n{}\n\nInstruction: {}", input_text, custom),
            _ => input_text.to_string(),
        };

        match &self.provider {
            AiProvider::Ollama { endpoint, model } => {
                let url = format!("{}/api/generate", endpoint.trim_end_matches('/'));
                let body = serde_json::json!({
                    "model": model,
                    "system": system_prompt,
                    "prompt": user_prompt,
                    "stream": false
                });

                let res = self.http_client.post(&url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| format!("Ollama connection error: {}", e))?;

                if !res.status().is_success() {
                    return Err(format!("Ollama returned HTTP {}", res.status()));
                }

                let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
                Ok(json["response"].as_str().unwrap_or("").to_string())
            }
            AiProvider::Groq { api_key, model } => {
                let url = "https://api.groq.com/openai/v1/chat/completions";
                let body = serde_json::json!({
                    "model": model,
                    "messages": [
                        {"role": "system", "content": system_prompt},
                        {"role": "user", "content": user_prompt}
                    ],
                    "temperature": 0.3
                });

                let res = self.http_client.post(url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| format!("Groq API error: {}", e))?;

                if !res.status().is_success() {
                    let err_text = res.text().await.unwrap_or_default();
                    return Err(format!("Groq API failed: {}", err_text));
                }

                let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
                let content = json["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
                Ok(content)
            }
            AiProvider::Gemini { api_key, model } => {
                let url = format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                    model, api_key
                );

                let body = serde_json::json!({
                    "systemInstruction": {
                        "parts": [{"text": system_prompt}]
                    },
                    "contents": [{
                        "parts": [{"text": user_prompt}]
                    }]
                });

                let res = self.http_client.post(&url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| format!("Gemini API error: {}", e))?;

                if !res.status().is_success() {
                    let err_text = res.text().await.unwrap_or_default();
                    return Err(format!("Gemini API failed: {}", err_text));
                }

                let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
                let text = json["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or("").to_string();
                Ok(text)
            }
            AiProvider::OpenAiCompatible { base_url, api_key, model } => {
                let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
                let body = serde_json::json!({
                    "model": model,
                    "messages": [
                        {"role": "system", "content": system_prompt},
                        {"role": "user", "content": user_prompt}
                    ]
                });

                let mut req = self.http_client.post(&url).json(&body);
                if !api_key.is_empty() {
                    req = req.header("Authorization", format!("Bearer {}", api_key));
                }

                let res = req.send().await.map_err(|e| format!("OpenAI API error: {}", e))?;
                if !res.status().is_success() {
                    let err_text = res.text().await.unwrap_or_default();
                    return Err(format!("OpenAI API failed: {}", err_text));
                }

                let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
                let content = json["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
                Ok(content)
            }
        }
    }
}

/// Hand-written fake AI service for reliable, offline unit tests without network calls
#[derive(Default)]
pub struct FakeAiLensService;

impl AiLensService for FakeAiLensService {
    fn process(&self, action: &LensAction, text: &str) -> Result<String, String> {
        match action {
            LensAction::Explain => Ok(format!("[Fake Explaining]: {}", text)),
            LensAction::TranslateThEn => Ok(format!("[Fake Translation]: {}", text)),
            LensAction::FixCode => Ok(format!("// Fixed Code:\n{}", text)),
            LensAction::PolishTone => Ok(format!("[Polished]: {}", text)),
            LensAction::Custom(c) => Ok(format!("[Prompt: {}] Result for {}", c, text)),
        }
    }
}
