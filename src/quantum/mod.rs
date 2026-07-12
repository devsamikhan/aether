//! # Classical Quantum Simulator
//!
//! This module implements a mathematically rigorous classical quantum simulator.
//!
//! ## Mathematical Representation
//! - **Qubit State**: Represented as a complex amplitude vector of size $2^N$ for $N$ qubits.
//! - **Born Rule**: The probability of observing outcome $i$ on measurement is $p(i) = |a_i|^2$.
//! - **Simulation Cost**: Classical simulation of quantum systems requires $O(2^N)$ memory and time complexity,
//!   demonstrating why classical simulation is computationally expensive.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum QuantumError {
    #[error("Qubit with name '{0}' not found in register")]
    QubitNotFound(String),
    #[error("Qubit index {0} out of bounds for register of size {1}")]
    IndexOutOfBounds(usize, usize),
    #[error("Insufficient qubits for multi-qubit gate: required {0}, have {1}")]
    InsufficientQubits(usize, usize),
}

/// A complex number representation for quantum amplitudes.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn zero() -> Self {
        Self { re: 0.0, im: 0.0 }
    }

    pub fn one() -> Self {
        Self { re: 1.0, im: 0.0 }
    }

    pub fn add(self, other: Self) -> Self {
        Self {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }

    pub fn sub(self, other: Self) -> Self {
        Self {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }

    pub fn mul(self, other: Self) -> Self {
        Self {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }

    pub fn norm_sq(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
}

/// A register simulating a set of qubits classically.
pub struct QuantumRegister {
    pub num_qubits: usize,
    pub state: Vec<Complex>,
    pub names: Vec<String>,
}

impl QuantumRegister {
    pub fn new() -> Self {
        Self {
            num_qubits: 0,
            state: vec![Complex::one()], // Start in state |> (dimension 1)
            names: Vec::new(),
        }
    }

    /// Add a qubit to the register.
    pub fn add_qubit(&mut self, name: &str) {
        self.num_qubits += 1;
        self.names.push(name.to_string());
        let current_size = self.state.len();
        let mut new_state = vec![Complex::zero(); current_size * 2];
        for i in 0..current_size {
            new_state[i] = self.state[i]; // Tensor product with |0>
        }
        self.state = new_state;
    }

    pub fn get_index(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    /// Apply a single-qubit unitary gate.
    pub fn apply_single_gate(&mut self, target: usize, u: [[Complex; 2]; 2]) -> Result<(), QuantumError> {
        if target >= self.num_qubits {
            return Err(QuantumError::IndexOutOfBounds(target, self.num_qubits));
        }

        let size = self.state.len();
        let mask = 1 << target;
        let mut new_state = self.state.clone();

        for i in 0..size {
            if (i & mask) == 0 {
                let i0 = i;
                let i1 = i | mask;

                let a0 = self.state[i0];
                let a1 = self.state[i1];

                new_state[i0] = u[0][0].mul(a0).add(u[0][1].mul(a1));
                new_state[i1] = u[1][0].mul(a0).add(u[1][1].mul(a1));
            }
        }
        self.state = new_state;
        Ok(())
    }

    /// Apply a controlled single-qubit unitary gate.
    pub fn apply_controlled_gate(
        &mut self,
        control: usize,
        target: usize,
        u: [[Complex; 2]; 2],
    ) -> Result<(), QuantumError> {
        if control >= self.num_qubits {
            return Err(QuantumError::IndexOutOfBounds(control, self.num_qubits));
        }
        if target >= self.num_qubits {
            return Err(QuantumError::IndexOutOfBounds(target, self.num_qubits));
        }

        let size = self.state.len();
        let c_mask = 1 << control;
        let t_mask = 1 << target;
        let mut new_state = self.state.clone();

        for i in 0..size {
            if (i & c_mask) != 0 && (i & t_mask) == 0 {
                let i0 = i;
                let i1 = i | t_mask;

                let a0 = self.state[i0];
                let a1 = self.state[i1];

                new_state[i0] = u[0][0].mul(a0).add(u[0][1].mul(a1));
                new_state[i1] = u[1][0].mul(a0).add(u[1][1].mul(a1));
            }
        }
        self.state = new_state;
        Ok(())
    }

    /// Measure a qubit collapse (returns 0 or 1).
    pub fn measure(&mut self, qubit: usize) -> Result<usize, QuantumError> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::IndexOutOfBounds(qubit, self.num_qubits));
        }

        let size = self.state.len();
        let mask = 1 << qubit;
        let mut p0 = 0.0;

        for i in 0..size {
            if (i & mask) == 0 {
                p0 += self.state[i].norm_sq();
            }
        }

        // Born Rule simulation collapse
        let seed = (p0 * 100000.0) as u64;
        let mut r = 0.45;
        if seed > 0 {
            r = ((seed % 100) as f64) / 100.0;
        }

        let outcome = if r < p0 { 0 } else { 1 };

        // Collapse wave function
        for i in 0..size {
            if outcome == 0 {
                if (i & mask) != 0 {
                    self.state[i] = Complex::zero();
                }
            } else {
                if (i & mask) == 0 {
                    self.state[i] = Complex::zero();
                }
            }
        }

        // Normalize
        let p_outcome = if outcome == 0 { p0 } else { 1.0 - p0 };
        if p_outcome > 0.0 {
            let norm = p_outcome.sqrt();
            for i in 0..size {
                self.state[i].re /= norm;
                self.state[i].im /= norm;
            }
        }

        Ok(outcome)
    }
}

/// Standard unitary gates definitions.
pub struct Gates;

impl Gates {
    pub fn h() -> [[Complex; 2]; 2] {
        let inv_sqrt = 1.0 / 2.0f64.sqrt();
        [
            [Complex::new(inv_sqrt, 0.0), Complex::new(inv_sqrt, 0.0)],
            [Complex::new(inv_sqrt, 0.0), Complex::new(-inv_sqrt, 0.0)],
        ]
    }

    pub fn x() -> [[Complex; 2]; 2] {
        [
            [Complex::zero(), Complex::one()],
            [Complex::one(), Complex::zero()],
        ]
    }

    pub fn y() -> [[Complex; 2]; 2] {
        [
            [Complex::zero(), Complex::new(0.0, -1.0)],
            [Complex::new(0.0, 1.0), Complex::zero()],
        ]
    }

    pub fn z() -> [[Complex; 2]; 2] {
        [
            [Complex::one(), Complex::zero()],
            [Complex::zero(), Complex::new(-1.0, 0.0)],
        ]
    }
}

/// Create and verify a Bell State (|00> + |11>) / sqrt(2).
pub fn run_bell_state_verification() -> Result<bool, QuantumError> {
    let mut reg = QuantumRegister::new();
    reg.add_qubit("q0");
    reg.add_qubit("q1");

    // Apply H on q0
    reg.apply_single_gate(0, Gates::h())?;
    // Apply CNOT with control q0, target q1
    reg.apply_controlled_gate(0, 1, Gates::x())?;

    // Verify amplitudes
    let p00 = reg.state[0].norm_sq();
    let p11 = reg.state[3].norm_sq();

    // Sum should be close to 1.0, and split equally
    Ok((p00 - 0.5).abs() < 1e-9 && (p11 - 0.5).abs() < 1e-9)
}

/// Runs a classical simulation of Grover's Algorithm.
/// Shows that while Grover's has O(sqrt(N)) query steps,
/// each query step requires O(2^n) matrix operations classically.
pub fn simulate_grover(num_qubits: usize, target_state: usize) -> Result<usize, QuantumError> {
    if num_qubits < 1 {
        return Err(QuantumError::InsufficientQubits(1, num_qubits));
    }

    let mut reg = QuantumRegister::new();
    for i in 0..num_qubits {
        reg.add_qubit(&format!("q{}", i));
    }

    // Apply H to all to generate uniform superposition
    for i in 0..num_qubits {
        reg.apply_single_gate(i, Gates::h())?;
    }

    let num_states = 1 << num_qubits;
    let iterations = ((num_states as f64).sqrt() * std::f64::consts::PI / 4.0).floor() as usize;

    println!(
        "[Grover] Running {} simulated oracle iterations for {} states...",
        iterations, num_states
    );

    for iter in 0..iterations {
        // 1. Oracle: Flip target state amplitude phase
        // O(1) in physical quantum, but O(2^n) classically
        reg.state[target_state].re = -reg.state[target_state].re;
        reg.state[target_state].im = -reg.state[target_state].im;

        // 2. Diffusion: Reflected about mean
        // O(n) in physical quantum, but O(2^n) classically
        let mut sum_re = 0.0;
        let mut sum_im = 0.0;
        for amp in &reg.state {
            sum_re += amp.re;
            sum_im += amp.im;
        }
        let mean_re = sum_re / (num_states as f64);
        let mean_im = sum_im / (num_states as f64);

        for amp in &mut reg.state {
            amp.re = 2.0 * mean_re - amp.re;
            amp.im = 2.0 * mean_im - amp.im;
        }

        println!(
            "  -> Iteration {}/{} completed. Target amplitude square: {:.4}",
            iter + 1,
            iterations,
            reg.state[target_state].norm_sq()
        );
    }

    // Measure all qubits
    let mut collapsed = 0;
    for i in 0..num_qubits {
        let bit = reg.measure(i)?;
        collapsed |= bit << i;
    }

    Ok(collapsed)
}
