use aether::quantum::{Gates, QuantumRegister};
use criterion::{Criterion, criterion_group, criterion_main};

fn benchmark_bell_state(c: &mut Criterion) {
    c.bench_function("apply_bell_state_gates", |b| {
        b.iter(|| {
            let mut reg = QuantumRegister::new();
            reg.add_qubit("q0");
            reg.add_qubit("q1");
            let _ = reg.apply_single_gate(0, Gates::h());
            let _ = reg.apply_controlled_gate(0, 1, Gates::x());
        })
    });
}

fn benchmark_grover_simulation(c: &mut Criterion) {
    c.bench_function("grover_sim_3_qubits", |b| {
        b.iter(|| {
            let _ = aether::quantum::simulate_grover(3, 5);
        })
    });
}

criterion_group!(benches, benchmark_bell_state, benchmark_grover_simulation);
criterion_main!(benches);
