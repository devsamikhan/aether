//! # Computational Complexity Analyzer
//!
//! This module provides tools to analyze and visualize computational complexity classes.
//!
//! ## Classical vs. Quantum Speedups
//! - **Unstructured Search**: Classical $O(N)$ vs. Quantum Grover $O(\sqrt{N})$.
//! - **Prime Factorization**: Classical General Number Field Sieve $O(\exp(n^{1/3}))$ vs. Quantum Shor $O(n^3)$.
//! - **NP-Complete Problems**: There is no proven polynomial-time quantum algorithm for NP-Complete problems,
//!   refuting claims that quantum computers solve all NP-hard problems instantly.

pub struct ComplexityAnalyzer;

impl ComplexityAnalyzer {
    /// Print a detailed comparison table of classical vs quantum complexity.
    pub fn print_speedup_comparison() {
        println!(
            "================================================================================="
        );
        println!(
            "              HONEST COMPUTATIONAL COMPLEXITY SPEEDUP ANALYSIS                   "
        );
        println!(
            "================================================================================="
        );
        println!(
            "{:<20} | {:<25} | {:<25} | {:<10}",
            "Algorithm Type", "Classical Complexity", "Quantum Complexity", "Speedup"
        );
        println!(
            "---------------------------------------------------------------------------------"
        );
        println!(
            "{:<20} | {:<25} | {:<25} | {:<10}",
            "Search (Unstructured)", "O(N) [Linear]", "O(√N) [Grover]", "Quadratic"
        );
        println!(
            "{:<20} | {:<25} | {:<25} | {:<10}",
            "Factoring (Shor's)", "O(e^(c*n^(1/3))) [Exp]", "O(n^3) [Shor]", "Exponential"
        );
        println!(
            "{:<20} | {:<25} | {:<25} | {:<10}",
            "SAT (NP-Complete)", "O(2^n) [Exponential]", "O(2^(n/2)) [Grover]", "Quadratic"
        );
        println!(
            "{:<20} | {:<25} | {:<25} | {:<10}",
            "Sorting (QuickSort)", "O(N log N) [Average]", "O(N log N) [Limit]", "No Speedup"
        );
        println!(
            "================================================================================="
        );
        println!("\n[CS Education] Why P vs NP remains Unsolved:");
        println!("- P represents problems solvable in polynomial time: O(n^k).");
        println!(
            "- NP represents problems where a candidate solution can be verified in polynomial time."
        );
        println!(
            "- NP-Complete problems are the hardest problems in NP. If any NP-Complete problem has a"
        );
        println!("  polynomial-time solution, then P = NP.");
        println!(
            "- Quantum computing (BQP class) does NOT solve all NP-Complete problems in polynomial time."
        );
        println!(
            "  Grover's algorithm provides a quadratic speedup but remains exponential for NP-hard problems."
        );
    }
}
