use crate::models::ContentType;
use regex::Regex;
use std::sync::OnceLock;

static URL_REGEX: OnceLock<Regex> = OnceLock::new();
static API_KEY_REGEX: OnceLock<Regex> = OnceLock::new();
static JWT_REGEX: OnceLock<Regex> = OnceLock::new();
static CC_REGEX: OnceLock<Regex> = OnceLock::new();

pub struct Sanitizer;

impl Sanitizer {
    fn url_regex() -> &'static Regex {
        URL_REGEX.get_or_init(|| Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap())
    }

    fn api_key_regex() -> &'static Regex {
        API_KEY_REGEX.get_or_init(|| {
            Regex::new(r"(?i)(sk-[a-zA-Z0-9_\-]{20,}|ghp_[a-zA-Z0-9]{36}|AIza[0-9A-Za-z-_]{35}|Bearer\s+[a-zA-Z0-9_\-\.]{20,})").unwrap()
        })
    }

    fn jwt_regex() -> &'static Regex {
        JWT_REGEX.get_or_init(|| {
            Regex::new(r"^[A-Za-z0-9-_=]+\.[A-Za-z0-9-_=]+\.?[A-Za-z0-9-_.+/=]*$").unwrap()
        })
    }

    fn cc_regex() -> &'static Regex {
        CC_REGEX.get_or_init(|| Regex::new(r"\b(?:\d[ -]*?){13,19}\b").unwrap())
    }

    pub fn classify_and_filter(text: &str) -> ContentType {
        let trimmed = text.trim();

        // 1. Check for API keys and JWT tokens
        if Self::api_key_regex().is_match(trimmed) {
            return ContentType::Sensitive;
        }

        // Long JWT-like strings
        if trimmed.len() > 60 && Self::jwt_regex().is_match(trimmed) {
            return ContentType::Sensitive;
        }

        // 2. Check for credit cards with Luhn algorithm validation
        if let Some(m) = Self::cc_regex().find(trimmed) {
            let digits: String = m.as_str().chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() >= 13 && digits.len() <= 19 && Self::luhn_check(&digits) {
                return ContentType::Sensitive;
            }
        }

        // 3. Check for URL
        if Self::url_regex().is_match(trimmed) {
            return ContentType::Url;
        }

        // 4. Check for code syntax
        if let Some(lang) = Self::detect_code_language(trimmed) {
            return ContentType::Code { language: lang };
        }

        ContentType::Text
    }

    fn luhn_check(digits: &str) -> bool {
        let mut sum = 0;
        let mut alternate = false;
        for c in digits.chars().rev() {
            if let Some(mut d) = c.to_digit(10) {
                if alternate {
                    d *= 2;
                    if d > 9 {
                        d -= 9;
                    }
                }
                sum += d;
                alternate = !alternate;
            } else {
                return false;
            }
        }
        sum % 10 == 0
    }

    fn detect_code_language(text: &str) -> Option<String> {
        if text.lines().count() >= 2 || text.contains('{') || text.contains(';') || text.contains("=>") {
            if text.contains("fn ") || text.contains("let mut ") || text.contains("impl ") {
                return Some("rust".to_string());
            }
            if text.contains("def ") || (text.contains("import ") && text.contains(':')) {
                return Some("python".to_string());
            }
            if text.contains("const ") || text.contains("function ") || text.contains("console.log") {
                return Some("javascript".to_string());
            }
            if text.contains("<html") || text.contains("<div") || text.contains("</") {
                return Some("html".to_string());
            }
            if text.contains("SELECT ") || text.contains("INSERT INTO ") {
                return Some("sql".to_string());
            }
            if text.contains('{') && text.contains('}') && text.contains(';') {
                return Some("code".to_string());
            }
        }
        None
    }
}
