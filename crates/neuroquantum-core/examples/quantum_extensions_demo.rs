//! Quantum Extensions Demo
//!
//! Demonstrates QUBO, TFIM, and Quantum Parallel Tempering algorithms
//! for solving optimization problems.

use nalgebra::DMatrix;
use neuroquantum_core::quantum::quantum_parallel_tempering::{
    IsingHamiltonian, QuantumBackend, QuantumParallelTempering, QuantumParallelTemperingConfig,
};
use neuroquantum_core::quantum::qubo_quantum::{
    graph_coloring_problem, max_cut_problem, tsp_problem, QUBOConfig, QUBOSolver,
    QuboQuantumBackend,
};
use neuroquantum_core::quantum::tfim::{FieldSchedule, TFIMSolver, TransverseFieldConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== NeuroQuantumDB - Quantum Extensions Demo ===\n");

    // Demo 1: QUBO Solver - Max-Cut Problem
    demo_qubo_max_cut()?;
    println!();

    // Demo 2: QUBO Solver - Graph Coloring
    demo_qubo_graph_coloring()?;
    println!();

    // Demo 3: QUBO Solver - TSP
    demo_qubo_tsp()?;
    println!();

    // Demo 4: TFIM Solver - Ferromagnetic Model
    demo_tfim_ferromagnetic()?;
    println!();

    // Demo 5: TFIM Solver - Spin Glass
    demo_tfim_spin_glass()?;
    println!();

    // Demo 6: Parallel Tempering
    demo_parallel_tempering().await?;
    println!();

    // Demo 7: Comparison of Methods
    demo_method_comparison().await?;

    println!("\n=== Demo Completed Successfully! ===");
    Ok(())
}

/// Demo 1: QUBO solver on Max-Cut problem
fn demo_qubo_max_cut() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Demo 1: QUBO Max-Cut Problem");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Create a graph with 6 nodes
    let edges = vec![
        (0, 1, 2.0),
        (1, 2, 3.0),
        (2, 3, 2.0),
        (3, 4, 1.0),
        (4, 5, 2.0),
        (5, 0, 1.0),
        (0, 3, 1.5),
        (1, 4, 1.5),
        (2, 5, 1.5),
    ];

    println!("Graph: 6 nodes, {} edges", edges.len());

    let problem = max_cut_problem(&edges, 6)?;
    let solver = QUBOSolver::new();
    let solution = solver.solve_problem(&problem)?;

    println!("Solution: {:?}", solution.variables);
    println!("Energy: {:.4}", solution.energy);
    println!("Quality: {:.2}%", solution.quality * 100.0);
    println!("Iterations: {}", solution.iterations);
    println!("Time: {:.2}ms", solution.computation_time_ms);

    // Calculate cut value
    let cut_value: f64 = edges
        .iter()
        .filter(|(i, j, _)| solution.variables[*i] != solution.variables[*j])
        .map(|(_, _, w)| w)
        .sum();
    println!("Cut value: {cut_value:.2}");

    Ok(())
}

/// Demo 2: QUBO solver on Graph Coloring problem
fn demo_qubo_graph_coloring() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Demo 2: QUBO Graph Coloring Problem");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Create a triangle graph (requires 3 colors)
    let edges = vec![(0, 1), (1, 2), (2, 0)];
    let num_colors = 3;

    println!("Graph: Triangle (3 nodes), {num_colors} colors");

    let problem = graph_coloring_problem(&edges, 3, num_colors)?;

    let config = QUBOConfig {
        backend: QuboQuantumBackend::SimulatedQuantumAnnealing,
        max_iterations: 5000,
        ..Default::default()
    };

    let solver = QUBOSolver::with_config(config);
    let solution = solver.solve_problem(&problem)?;

    println!("Energy: {:.4}", solution.energy);
    println!("Quality: {:.2}%", solution.quality * 100.0);
    println!("Iterations: {}", solution.iterations);
    println!("Time: {:.2}ms", solution.computation_time_ms);

    // Decode coloring
    println!("Node colors:");
    for node in 0..3 {
        for color in 0..num_colors {
            let idx = node * num_colors + color;
            if solution.variables[idx] == 1 {
                println!("  Node {node}: Color {color}");
            }
        }
    }

    Ok(())
}

/// Demo 3: QUBO solver on TSP problem
fn demo_qubo_tsp() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗺️  Demo 3: QUBO Traveling Salesman Problem");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Create a small TSP instance (5 cities)
    let dist_matrix = DMatrix::from_row_slice(
        5,
        5,
        &[
            0.0, 2.0, 3.0, 4.0, 5.0, 2.0, 0.0, 4.0, 3.0, 2.0, 3.0, 4.0, 0.0, 2.0, 3.0, 4.0, 3.0,
            2.0, 0.0, 4.0, 5.0, 2.0, 3.0, 4.0, 0.0,
        ],
    );

    println!("Cities: 5");
    println!(
        "Distance matrix size: {}x{}",
        dist_matrix.nrows(),
        dist_matrix.ncols()
    );

    let problem = tsp_problem(&dist_matrix)?;

    let config = QUBOConfig {
        backend: QuboQuantumBackend::SimulatedQuantumAnnealing,
        max_iterations: 3000,
        ..Default::default()
    };

    let solver = QUBOSolver::with_config(config);
    let solution = solver.solve_problem(&problem)?;

    println!("Energy: {:.4}", solution.energy);
    println!("Quality: {:.2}%", solution.quality * 100.0);
    println!("Iterations: {}", solution.iterations);
    println!("Time: {:.2}ms", solution.computation_time_ms);

    Ok(())
}

/// Demo 4: TFIM solver on ferromagnetic model
fn demo_tfim_ferromagnetic() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧲 Demo 4: TFIM Ferromagnetic Model");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let problem = TFIMSolver::ferromagnetic_model(8, 1.0)?;

    let config = TransverseFieldConfig {
        initial_field: 10.0,
        final_field: 0.1,
        num_steps: 1000,
        field_schedule: FieldSchedule::Linear,
        quantum_tunneling: true,
        ..Default::default()
    };

    println!("Spins: {}", problem.num_spins);
    println!("Field schedule: Linear (10.0 → 0.1)");

    let solver = TFIMSolver::with_config(config);
    let solution = solver.solve(&problem)?;

    println!("Solution: {:?}", solution.spins);
    println!("Energy: {:.4}", solution.energy);
    println!(
        "Ground state probability: {:.6}",
        solution.ground_state_prob
    );
    println!("Tunneling events: {}", solution.tunneling_events);
    println!("Steps: {}", solution.steps);
    println!("Time: {:.2}ms", solution.computation_time_ms);

    // Check alignment
    let all_aligned = solution.spins.windows(2).all(|w| w[0] == w[1]);
    println!("All spins aligned: {all_aligned}");

    Ok(())
}

/// Demo 5: TFIM solver on spin glass model
fn demo_tfim_spin_glass() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌀 Demo 5: TFIM Spin Glass Model");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let problem = TFIMSolver::spin_glass_model(10, 2.0)?;

    let config = TransverseFieldConfig {
        initial_field: 15.0,
        final_field: 0.05,
        num_steps: 1500,
        field_schedule: FieldSchedule::Exponential { decay_rate: 3.0 },
        temperature: 0.5,
        quantum_tunneling: true,
    };

    println!("Spins: {}", problem.num_spins);
    println!("Field schedule: Exponential (decay=3.0)");
    println!("Disorder strength: 2.0");

    let solver = TFIMSolver::with_config(config);
    let solution = solver.solve(&problem)?;

    println!("Solution: {:?}", solution.spins);
    println!("Energy: {:.4}", solution.energy);
    println!(
        "Ground state probability: {:.6}",
        solution.ground_state_prob
    );
    println!("Tunneling events: {}", solution.tunneling_events);
    println!("Time: {:.2}ms", solution.computation_time_ms);

    Ok(())
}

/// Demo 6: Quantum Parallel Tempering
async fn demo_parallel_tempering() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔥 Demo 6: Quantum Parallel Tempering");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Create Ising problem
    let couplings = DMatrix::from_fn(10, 10, |i, j| {
        if i == j {
            0.0
        } else {
            (i as f64 - j as f64).abs().recip()
        }
    });
    let external_fields = vec![0.0; 10];

    let config = QuantumParallelTemperingConfig {
        num_replicas: 8,
        min_temperature: 0.5,
        max_temperature: 10.0,
        sweeps_per_exchange: 100,
        num_exchanges: 50,
        backend: QuantumBackend::PathIntegralMonteCarlo,
        adaptive_temperatures: false,
        ..Default::default()
    };

    println!("Spins: 10");
    println!("Replicas: {}", config.num_replicas);
    println!(
        "Temperature range: {:.1} - {:.1}",
        config.min_temperature, config.max_temperature
    );
    println!("Exchanges: {}", config.num_exchanges);
    println!("Backend: {:?}", config.backend);

    let mut qpt = QuantumParallelTempering::with_config(config);
    let hamiltonian = IsingHamiltonian::new(10, couplings, external_fields, 1.0);
    let initial_state = vec![1; 10];

    let solution = qpt.optimize(hamiltonian, initial_state).await?;

    println!("Best solution: {:?}", solution.best_configuration);
    println!("Best energy: {:.4}", solution.best_energy);
    println!("Best replica: {}", solution.best_replica_id);
    println!(
        "Exchange acceptance rate: {:.2}%",
        solution.acceptance_rate * 100.0
    );
    println!(
        "Total exchanges: {} ({} accepted)",
        solution.total_exchanges, solution.accepted_exchanges
    );
    println!("Time: {:.2}ms", solution.computation_time_ms);

    Ok(())
}

/// Demo 7: Method comparison
async fn demo_method_comparison() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚖️  Demo 7: Method Comparison");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Create the same problem for all methods
    let edges = vec![
        (0, 1, 1.0),
        (1, 2, 1.0),
        (2, 3, 1.0),
        (3, 0, 1.0),
        (0, 2, 1.0),
        (1, 3, 1.0),
    ];

    println!("Problem: Max-Cut on 4-node graph with 6 edges\n");

    // Method 1: QUBO without tunneling
    println!("1. QUBO (classical fallback):");
    let problem = max_cut_problem(&edges, 4)?;
    let config = QUBOConfig {
        backend: QuboQuantumBackend::ClassicalFallback,
        max_iterations: 1000,
        ..Default::default()
    };
    let solver = QUBOSolver::with_config(config);
    let sol1 = solver.solve_problem(&problem)?;
    println!(
        "   Energy: {:.4}, Quality: {:.1}%, Time: {:.2}ms",
        sol1.energy,
        sol1.quality * 100.0,
        sol1.computation_time_ms
    );

    // Method 2: QUBO with tunneling
    println!("2. QUBO (simulated quantum annealing):");
    let config = QUBOConfig {
        backend: QuboQuantumBackend::SimulatedQuantumAnnealing,
        max_iterations: 1000,
        ..Default::default()
    };
    let solver = QUBOSolver::with_config(config);
    let sol2 = solver.solve_problem(&problem)?;
    println!(
        "   Energy: {:.4}, Quality: {:.1}%, Time: {:.2}ms",
        sol2.energy,
        sol2.quality * 100.0,
        sol2.computation_time_ms
    );

    // Method 3: TFIM
    println!("3. TFIM (Transverse Field Ising Model):");
    let tfim_problem = TFIMSolver::from_qubo(&problem.q_matrix)?;
    let solver = TFIMSolver::new();
    let sol3 = solver.solve(&tfim_problem)?;
    println!(
        "   Energy: {:.4}, Tunneling: {}, Time: {:.2}ms",
        sol3.energy, sol3.tunneling_events, sol3.computation_time_ms
    );

    // Method 4: Quantum Parallel Tempering
    println!("4. Quantum Parallel Tempering (PIMC):");
    let couplings = DMatrix::from_fn(4, 4, |i, j| if i == j { 0.0 } else { 1.0 });
    let external_fields = vec![0.0; 4];
    let config = QuantumParallelTemperingConfig {
        num_replicas: 4,
        num_exchanges: 20,
        sweeps_per_exchange: 50,
        backend: QuantumBackend::PathIntegralMonteCarlo,
        ..Default::default()
    };
    let mut qpt = QuantumParallelTempering::with_config(config);
    let hamiltonian = IsingHamiltonian::new(4, couplings, external_fields, 1.0);
    let sol4 = qpt.optimize(hamiltonian, vec![1, -1, 1, -1]).await?;
    println!(
        "   Energy: {:.4}, Acceptance: {:.1}%, Time: {:.2}ms",
        sol4.best_energy,
        sol4.acceptance_rate * 100.0,
        sol4.computation_time_ms
    );

    println!("\n📈 Summary:");
    println!(
        "   Best energy: {:.4} (Method {})",
        [sol1.energy, sol2.energy, sol3.energy, sol4.best_energy]
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, e)| (i + 1, e))
            .unwrap()
            .1,
        [sol1.energy, sol2.energy, sol3.energy, sol4.best_energy]
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i + 1)
            .unwrap()
    );

    Ok(())
}
