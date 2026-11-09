use omni_clipboard::models::{ClipItem, ContentType, SearchQuery};
use omni_clipboard::sanitizer::Sanitizer;
use omni_clipboard::storage::{ClipStorage, FakeStorage, SqliteStorage};

#[test]
fn test_models_hash_and_preview() {
    let content = "Hello World! This is a test clip for Omni-Clipboard.\nSecond line should not be in short preview.";
    let item = ClipItem::new(content, ContentType::Text, None);

    assert_eq!(item.char_count, content.chars().count());
    assert_eq!(item.preview_text, "Hello World! This is a test clip for Omni-Clipboard.");
    assert_eq!(item.content_hash, ClipItem::compute_hash(content));
    assert!(!item.is_pinned);
}

#[test]
fn test_sanitizer_detection() {
    // API keys
    assert_eq!(
        Sanitizer::classify_and_filter("sk-proj-1234567890abcdef1234567890abcdef12345678"),
        ContentType::Sensitive
    );
    assert_eq!(
        Sanitizer::classify_and_filter("Bearer ghp_abcdefghijklmnopqrstuvwxyz1234567890"),
        ContentType::Sensitive
    );

    // URL
    assert_eq!(
        Sanitizer::classify_and_filter("https://github.com/heilcheng/awesome-agent-skills"),
        ContentType::Url
    );

    // Code
    assert_eq!(
        Sanitizer::classify_and_filter("fn main() {\n    println!(\"hello\");\n}"),
        ContentType::Code { language: "rust".into() }
    );
    assert_eq!(
        Sanitizer::classify_and_filter("def calculate_sum(a, b):\n    return a + b"),
        ContentType::Code { language: "python".into() }
    );
    assert_eq!(
        Sanitizer::classify_and_filter("const greeting = 'hello';\nconsole.log(greeting);"),
        ContentType::Code { language: "javascript".into() }
    );

    // Plain text
    assert_eq!(
        Sanitizer::classify_and_filter("Buy some groceries from the supermarket"),
        ContentType::Text
    );
}

#[test]
fn test_storage_crud_and_deduplication() {
    let storage = SqliteStorage::in_memory().expect("in-memory db failed");

    // 1. Save new clip
    let clip1 = ClipItem::new("Unique text 1", ContentType::Text, Some("TestApp".into()));
    let id1 = storage.save_clip(clip1.clone()).expect("save failed");
    assert_eq!(storage.count().unwrap(), 1);

    // 2. Save duplicate content -> must deduplicate (count stays 1)
    let clip_dup = ClipItem::new("Unique text 1", ContentType::Text, None);
    let id_dup = storage.save_clip(clip_dup).expect("save dup failed");
    assert_eq!(id1, id_dup);
    assert_eq!(storage.count().unwrap(), 1);

    // 3. Save second unique item
    let clip2 = ClipItem::new("Unique text 2", ContentType::Text, None);
    let id2 = storage.save_clip(clip2).expect("save clip2 failed");
    assert_eq!(storage.count().unwrap(), 2);

    // 4. Pin item 2
    assert!(storage.set_pinned(&id2, true).unwrap());
    let fetched2 = storage.get_clip(&id2).unwrap().unwrap();
    assert!(fetched2.is_pinned);

    // 5. Delete item 1
    assert!(storage.delete_clip(&id1).unwrap());
    assert_eq!(storage.count().unwrap(), 1);
    assert!(storage.get_clip(&id1).unwrap().is_none());
}

#[test]
fn test_search_and_ordering() {
    let storage = SqliteStorage::in_memory().expect("in-memory db failed");

    storage.save_clip(ClipItem::new("Rust compiler optimization guide", ContentType::Text, None)).unwrap();
    storage.save_clip(ClipItem::new("Python machine learning tutorial", ContentType::Text, None)).unwrap();
    let id_pinned = storage.save_clip(ClipItem::new("Rust memory safety internals", ContentType::Text, None)).unwrap();
    storage.set_pinned(&id_pinned, true).unwrap();

    // Search for "Rust" -> pinned item should be first
    let results = storage.search_clips(&SearchQuery {
        query: "Rust".into(),
        pinned_only: false,
        limit: 10,
    }).unwrap();

    assert_eq!(results.len(), 2);
    assert!(results[0].is_pinned);
    assert_eq!(results[0].id, id_pinned);

    // Empty search returns all ordered by pinned then date
    let all = storage.search_clips(&SearchQuery::default()).unwrap();
    assert_eq!(all.len(), 3);
    assert!(all[0].is_pinned);
}

#[test]
fn test_fake_storage_contract_parity() {
    let fake = FakeStorage::new();

    let clip = ClipItem::new("Fake storage item", ContentType::Text, None);
    let id = fake.save_clip(clip.clone()).unwrap();
    assert_eq!(fake.count().unwrap(), 1);

    // Deduplication check
    let dup = ClipItem::new("Fake storage item", ContentType::Text, None);
    let dup_id = fake.save_clip(dup).unwrap();
    assert_eq!(id, dup_id);
    assert_eq!(fake.count().unwrap(), 1);

    // Search
    let res = fake.search_clips(&SearchQuery {
        query: "storage".into(),
        pinned_only: false,
        limit: 10,
    }).unwrap();
    assert_eq!(res.len(), 1);
}

#[test]
fn test_ai_lens_action_prompts_and_fake_service() {
    use omni_clipboard::ai_lens::{AiLensService, FakeAiLensService, LensAction};

    let fake_ai = FakeAiLensService;

    // Explain
    let explain_res = fake_ai.process(&LensAction::Explain, "Rust is a systems programming language.").unwrap();
    assert!(explain_res.contains("Rust is a systems programming language"));

    // Translation
    let trans_res = fake_ai.process(&LensAction::TranslateThEn, "Hello world").unwrap();
    assert!(trans_res.contains("Hello world"));

    // Fix Code
    let code_res = fake_ai.process(&LensAction::FixCode, "let x = 10").unwrap();
    assert!(code_res.contains("let x = 10"));

    // Custom prompt
    let custom_res = fake_ai.process(&LensAction::Custom("Summarize".into()), "Big document").unwrap();
    assert!(custom_res.contains("Summarize"));
    assert!(custom_res.contains("Big document"));
}
