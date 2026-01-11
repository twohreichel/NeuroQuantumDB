//! # TFIM Quantum Annealing Example
//!
//! This example demonstrates using real quantum annealing hardware (D-Wave, AWS Braket)
//! to solve Transverse Field Ising Model (TFIM) problems.
//!
//! ## Setup
//!
//! ### D-Wave Configuration
//! Set the following environment variables:
//! ```bash
//! export DWAVE_API_TOKEN="your-api-token-here"
//! export DWAVE_SOLVER="Advantage_system6.4"  # Optional
//! ```
//!
//! ### AWS Braket Configuration
//! Set AWS credentials:
//! ```bash
//! export AWS_ACCESS_KEY_ID="your-access-key"
//! export AWS_SECRET_ACCESS_KEY="your-secret-key"
//! export AWS_REGION="us-west-1"
//! export BRAKET_DEVICE_ARN="arn:aws:braket:::device/qpu/d-wave/Advantage_system6"
//! ```
//!
//! ## Running
//! ```bash
//! cargo run --example tfim_quantum_annealing_demo
//! ```

use nalgebra::DMatrix;
use neuroquantum_core::quantum::{
    AnnealingBackend, BraketTFIMConfig, BraketTFIMSolver, DWaveTFIMConfig, DWaveTFIMSolver,
    TFIMBackendPreference, TFIMProblem, UnifiedTFIMAnnealingConfig, UnifiedTFIMAnnealingSolver,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("🧲 TFIM Quantum Annealing Demo");
    println!("================================\n");

    // Example 1: Ferromagnetic Ising Model
    println!("📊 Example 1: Ferromagnetic Chain (J > 0)");
    println!("Problem: Find ground state of 4-spin ferromagnetic chain");
    let ferromagnetic_problem = create_ferromagnetic_chain(4, 2.0);
    demo_unified_solver(&ferromagnetic_problem, "Ferromagnetic").await?;

    println!("\n---\n");

    // Example 2: Antiferromagnetic Ising Model
    println!("📊 Example 2: Antiferromagnetic Chain (J < 0)");
    println!("Problem: Find ground state of 4-spin antiferromagnetic chain");
    let antiferromagnetic_problem = create_antiferromagnetic_chain(4, -1.5);
    demo_unified_solver(&antiferromagnetic_problem, "Antiferromagnetic").await?;

    println!("\n---\n");

    // Example 3: Frustrated Spin System
    println!("📊 Example 3: Frustrated Triangle");
    println!("Problem: 3-spin frustrated system (no configuration satisfies all bonds)");
    let frustrated_problem = create_frustrated_triangle();
    demo_unified_solver(&frustrated_problem, "Frustrated").await?;

    println!("\n---\n");

    // Example 4: Spin Glass with External Fields
    println!("📊 Example 4: Spin Glass with External Fields");
    println!("Problem: 5-spin system with random couplings and external fields");
    let spin_glass_problem = create_spin_glass_with_fields(5);
    demo_unified_solver(&spin_glass_problem, "SpinGlass").await?;

    println!("\n---\n");

    // Example 5: Specific Backend Selection
    println!("📊 Example 5: Using Specific Backends");
    demo_specific_backends(&ferromagnetic_problem).await?;

    println!("\n✅ Demo completed!");
    println!("\n💡 Tips:");
    println!("   - Set DWAVE_API_TOKEN for real D-Wave hardware");
    println!("   - Set AWS credentials for AWS Braket access");
    println!("   - Without credentials, falls back to classical simulation");
    println!("   - Use TFIM_BACKEND env var to select backend: dwave, braket, classical, auto");

    Ok(())
}

/// Demonstrate unified solver with automatic backend selection
async fn demo_unified_solver(
    problem: &TFIMProblem,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("  Solving using UnifiedTFIMAnnealingSolver (auto-select)...");

    let solver = UnifiedTFIMAnnealingSolver::from_env();
    let solution = solver.solve(problem).await?;

    println!("  ✓ Solution found:");
    println!("    Spins: {:?}", solution.spins);
    println!("    Energy: {:.6}", solution.energy);
    println!(
        "    Ground state probability: {:.4}",
        solution.ground_state_prob
    );
    println!(
        "    Computation time: {:.2} ms",
        solution.computation_time_ms
    );
    println!("    Tunneling events: {}", solution.tunneling_events);

    // Analyze solution
    analyze_solution(&solution, name);

    Ok(())
}

/// Demonstrate using specific backends
async fn demo_specific_backends(problem: &TFIMProblem) -> Result<(), Box<dyn std::error::Error>> {
    println!("  Testing D-Wave backend:");
    let dwave_config = DWaveTFIMConfig {
        api_token: std::env::var("DWAVE_API_TOKEN").ok(),
        num_reads: 100,
        annealing_time_us: 20,
        ..Default::default()
    };
    let dwave_solver = DWaveTFIMSolver::new(dwave_config);

    if dwave_solver.is_available() {
        println!("    ✓ D-Wave API available");
        let solution = dwave_solver.solve(problem).await?;
        println!(
            "    Energy: {:.6}, Time: {:.2} ms",
            solution.energy, solution.computation_time_ms
        );
    } else {
        println!("    ⚠ D-Wave API not configured (will use classical fallback)");
        let solution = dwave_solver.solve(problem).await?;
        println!(
            "    Energy (classical): {:.6}, Time: {:.2} ms",
            solution.energy, solution.computation_time_ms
        );
    }

    println!("\n  Testing AWS Braket backend:");
    let braket_config = BraketTFIMConfig {
        num_shots: 100,
        ..Default::default()
    };
    let braket_solver = BraketTFIMSolver::new(braket_config);

    if braket_solver.is_available() {
        println!("    ✓ AWS Braket available");
        let solution = braket_solver.solve(problem).await?;
        println!(
            "    Energy: {:.6}, Time: {:.2} ms",
            solution.energy, solution.computation_time_ms
        );
    } else {
        println!("    ⚠ AWS Braket not configured (will use classical fallback)");
        let solution = braket_solver.solve(problem).await?;
        println!(
            "    Energy (classical): {:.6}, Time: {:.2} ms",
            solution.energy, solution.computation_time_ms
        );
    }

    println!("\n  Testing explicit classical backend:");
    let classical_config = UnifiedTFIMAnnealingConfig {
        preference: TFIMBackendPreference::Classical,
        dwave_config: None,
        braket_config: None,
        classical_config: Default::default(),
    };
    let classical_solver = UnifiedTFIMAnnealingSolver::new(classical_config);
    let solution = classical_solver.solve(problem).await?;
    println!(
        "    Energy: {:.6}, Time: {:.2} ms",
        solution.energy, solution.computation_time_ms
    );

    Ok(())
}

/// Create a ferromagnetic chain (all couplings positive)
fn create_ferromagnetic_chain(n: usize, coupling: f64) -> TFIMProblem {
    TFIMProblem {
        num_spins: n,
        couplings: DMatrix::from_fn(n, n, |i, j| {
            if i != j && (i as i32 - j as i32).abs() == 1 {
                coupling
            } else {
                0.0
            }
        }),
        external_fields: vec![0.0; n],
        name: "Ferromagnetic_Chain".to_string(),
    }
}

/// Create an antiferromagnetic chain (all couplings negative)
fn create_antiferromagnetic_chain(n: usize, coupling: f64) -> TFIMProblem {
    TFIMProblem {
        num_spins: n,
        couplings: DMatrix::from_fn(n, n, |i, j| {
            if i != j && (i as i32 - j as i32).abs() == 1 {
                coupling
            } else {
                0.0
            }
        }),
        external_fields: vec![0.0; n],
        name: "Antiferromagnetic_Chain".to_string(),
    }
}

/// Create a frustrated triangle (3 spins with competing interactions)
fn create_frustrated_triangle() -> TFIMProblem {
    let mut couplings = DMatrix::zeros(3, 3);
    // All negative couplings create frustration in triangle
    couplings[(0, 1)] = -1.0;
    couplings[(1, 0)] = -1.0;
    couplings[(1, 2)] = -1.0;
    couplings[(2, 1)] = -1.0;
    couplings[(0, 2)] = -1.0;
    couplings[(2, 0)] = -1.0;

    TFIMProblem {
        num_spins: 3,
        couplings,
        external_fields: vec![0.0; 3],
        name: "Frustrated_Triangle".to_string(),
    }
}

/// Create a spin glass with random couplings and external fields
fn create_spin_glass_with_fields(n: usize) -> TFIMProblem {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Create symmetric coupling matrix
    let mut couplings = DMatrix::zeros(n, n);
    for i in 0..n {
        for j in (i + 1)..n {
            let coupling = (rng.gen::<f64>() - 0.5) * 2.0;
            couplings[(i, j)] = coupling;
            couplings[(j, i)] = coupling; // Make symmetric
        }
    }

    let external_fields: Vec<f64> = (0..n).map(|_| (rng.gen::<f64>() - 0.5) * 1.0).collect();

    TFIMProblem {
        num_spins: n,
        couplings,
        external_fields,
        name: "SpinGlass_With_Fields".to_string(),
    }
}

/// Analyze the solution and provide insights
fn analyze_solution(solution: &neuroquantum_core::quantum::TFIMSolution, problem_type: &str) {
    let n = solution.spins.len();

    // Calculate magnetization
    let magnetization: f64 = solution.spins.iter().map(|&s| s as f64).sum::<f64>() / n as f64;

    // Check if all spins are aligned
    let all_up = solution.spins.iter().all(|&s| s == 1);
    let all_down = solution.spins.iter().all(|&s| s == -1);
    let aligned = all_up || all_down;

    println!("    Analysis:");
    println!("      Magnetization: {:.3}", magnetization);
    println!("      All aligned: {}", if aligned { "Yes" } else { "No" });

    match problem_type {
        "Ferromagnetic" => {
            if aligned {
                println!("      ✓ Correct: Ferromagnetic prefers aligned spins");
            } else {
                println!("      ⚠ Unexpected: Not all spins aligned for ferromagnet");
            }
        }
        "Antiferromagnetic" => {
            let alternating = solution.spins.windows(2).all(|w| w[0] != w[1]);
            if alternating {
                println!("      ✓ Correct: Antiferromagnetic prefers alternating spins");
            } else if !aligned {
                println!("      ✓ Shows anti-alignment pattern");
            }
        }
        "Frustrated" => {
            println!("      Note: Frustration means no configuration satisfies all bonds");
            println!("      Energy should be higher than unfrustrated systems");
        }
        "SpinGlass" => {
            println!("      Note: Spin glass has complex energy landscape");
            println!("      Multiple local minima possible");
        }
        _ => {}
    }
}
