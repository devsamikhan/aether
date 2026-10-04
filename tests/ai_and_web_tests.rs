use aether::codegen::web_builder::WebBuilder;
use aether::vm::run_source;
use std::fs;

#[test]
fn test_ai_module_intelligence_and_agent() {
    let source = r#"
# 1. AI completion
let slogan = AI.complete("Write a catchy slogan for AETHER", "aether-reasoner")
assert(Strings.starts_with(slogan, "⚡"))

let explanation = AI.complete("Explain intent-driven abstractions")
assert(Strings.starts_with(explanation, "Intelligence Insight:"))

# 2. AI sentiment analysis
let pos = AI.sentiment("AETHER is incredibly fast and I love the developer experience!")
assert(pos["sentiment"] == "positive")
assert(pos["score"] > 0.9)

let neg = AI.sentiment("The legacy compiler had a slow bug and failed to build.")
assert(neg["sentiment"] == "negative")

# 3. AI Autonomous Agent
let agent = AI.agent("CodeSentinel", "Security Auditor", "Audit code for memory safety")
assert(agent["name"] == "CodeSentinel")
assert(agent["role"] == "Security Auditor")

let task_res = AI.ask(agent, "Scan smart contract for reentrancy bugs")
assert(Strings.starts_with(task_res, "[Agent: CodeSentinel"))

# 4. AI Embeddings (16-dimensional vector)
let emb = AI.embeddings("Aether Universal Runtime")
assert(len(emb) == 16)
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "AI module test failed: {:?}", res.err());
}

#[test]
fn test_web_builder_standalone_html_bundle() {
    let temp_dir = tempfile::tempdir().unwrap();
    let source_path = temp_dir.path().join("web_app.ae");
    let output_html = temp_dir.path().join("index.html");

    let source = r#"
# AETHER Web App
let app_title = "My Web Portal"
print(f"Starting {app_title} in browser!")
Mobile.show_toast("Welcome to Web Portal")
"#;
    fs::write(&source_path, source).unwrap();

    let builder = WebBuilder::new("Web Portal");
    let res = builder.build_web_app(&source_path, &output_html);
    assert!(res.is_ok(), "Web build failed: {:?}", res.err());

    assert!(output_html.exists());
    let html_content = fs::read_to_string(&output_html).unwrap();
    assert!(html_content.contains("<!DOCTYPE html>"));
    assert!(html_content.contains("AETHER Web"));
    assert!(html_content.contains("runAetherApp()"));
    assert!(html_content.contains("Mobile"));
    assert!(html_content.contains("console-output"));
}
