// ==============================================================================
// AETHER 2.0 Tour of AETHER & Flagship Showcase Verification Tests
// ==============================================================================

use aether::tour::{run_tour_lesson, TOUR_LESSONS};
use std::fs;

#[test]
fn test_all_tour_lessons_execute_successfully() {
    assert_eq!(TOUR_LESSONS.len(), 7, "Tour must have 7 core lessons");

    for lesson in TOUR_LESSONS {
        let result = run_tour_lesson(lesson.id);
        assert!(
            result.is_ok(),
            "Tour lesson {} ('{}') failed to execute: {:?}",
            lesson.id,
            lesson.title,
            result.err()
        );
    }
}

#[test]
fn test_flagship_aetherbrain_showcase_execution() {
    let source_path = "examples/44_aether_brain_autonomous_agent.ae";
    let source_code = fs::read_to_string(source_path)
        .expect("Failed to read 44_aether_brain_autonomous_agent.ae");

    let result = aether::vm::run_source(&source_code);
    assert!(
        result.is_ok(),
        "AetherBrain flagship showcase execution failed: {:?}",
        result.err()
    );
}
