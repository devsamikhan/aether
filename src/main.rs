#![allow(dead_code, unused_imports, unused_variables)]

mod cli;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use aether::quantum::{Complex, Gates, QuantumRegister};

    #[test]
    fn test_complex_operations() {
        let c1 = Complex::new(1.0, 2.0);
        let c2 = Complex::new(3.0, -4.0);
        let sum = c1.add(c2);
        assert_eq!(sum.re, 4.0);
        assert_eq!(sum.im, -2.0);
        let prod = c1.mul(c2);
        assert_eq!(prod.re, 11.0);
        assert_eq!(prod.im, 2.0);
    }

    #[test]
    fn test_quantum_hadamard() {
        let mut reg = QuantumRegister::new();
        reg.add_qubit("q0");
        reg.apply_single_gate(0, Gates::h()).unwrap();
        let p0 = reg.state[0].norm_sq();
        let p1 = reg.state[1].norm_sq();
        assert!((p0 - 0.5).abs() < 1e-9);
        assert!((p1 - 0.5).abs() < 1e-9);
    }
}
