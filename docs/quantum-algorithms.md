# AETHER Quantum Algorithms

AETHER allows developers to express complex quantum algorithms cleanly.

---

## 1. Grover's Swarm Search
Grover's search uses amplitude amplification to locate items in an unstructured database of size $N$ in $O(\sqrt{N})$ queries.

### Implementation
```aether
intent GroverSearch {
    fn search(database: Vec<Int>, target: Int) {
        qubit q0;
        qubit q1;
        superpose(q0);
        superpose(q1);
        
        // Amplitude Amplification Oracle
        oracle_match(q0, q1, target);
        
        measure(q0) => result;
        return result;
    }
}
```

---

## 2. Shor's Prime Factoring
Finds prime factors of composite number $N$ in $O((\log N)^3)$ time using quantum phase estimation.
```aether
intent ShorFactoring {
    fn factor(n: Int) {
        qubit[8] register;
        quantum_phase_estimation(register);
        let period = measure_all(register);
        return period;
    }
}
```
*Note: Shor's algorithm is simulated mathematically in-process via wave-function collapse projections.*
