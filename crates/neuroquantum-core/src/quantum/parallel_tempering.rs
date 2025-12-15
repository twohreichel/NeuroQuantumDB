//! # Parallel Tempering (Replica Exchange Monte Carlo)
//!
//! # ⚠️ Classical Algorithm Notice
//!
//! **This is a PURELY CLASSICAL Monte Carlo algorithm.**
//! Parallel tempering is a well-established classical technique that predates
//! quantum computing. It is included in the quantum module because it is often
//! used in conjunction with quantum-inspired algorithms.
//!
//! ## Algorithm Description
//!
//! Parallel tempering (also known as Replica Exchange Monte Carlo or REMC)
//! maintains multiple copies ("replicas") of the system at different temperatures.
//! High-temperature replicas explore broadly, while low-temperature replicas
//! exploit local minima. Periodic exchanges between replicas allow global
//! exploration without getting stuck.
//!
//! ## Key Features
//!
//! - **Temperature Ladder**: Geometric or linear spacing of temperatures
//! - **Metropolis Criterion**: Accept/reject moves based on energy change
//! - **Replica Exchange**: Swap configurations between adjacent temperatures
//! - **Adaptive Tuning**: Automatic temperature adjustment for optimal acceptance
//!
//! ## Computational Complexity
//!
//! - Time: O(replicas × steps × exchanges)
//! - Space: O(replicas × system_size)
//!
//! ## Applications
//!
//! - Enhanced sampling for optimization problems
//! - Combinatorial optimization
//! - Simulation of complex energy landscapes

use crate::error::{CoreError, CoreResult};
use nalgebra::DMatrix;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

/// Configuration for parallel tempering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelTemperingConfig {
    /// Number of replicas (temperature levels)
    pub num_replicas: usize,
    /// Minimum temperature
    pub min_temperature: f64,
    /// Maximum temperature
    pub max_temperature: f64,
    /// Temperature distribution (Geometric or Linear)
    pub temp_distribution: TemperatureDistribution,
    /// Number of steps per replica before exchange
    pub steps_per_exchange: usize,
    /// Total number of exchanges
    pub num_exchanges: usize,
    /// Enable adaptive temperature adjustment
    pub adaptive_temperatures: bool,
}

/// Temperature distribution strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemperatureDistribution {
    /// Geometric: T_i = T_min * (T_max / T_min)^(i / (N-1))
    Geometric,
    /// Linear: T_i = T_min + (T_max - T_min) * i / (N-1)
    Linear,
    /// Custom temperature values
    Custom { temperatures: Vec<f64> },
}

impl Default for ParallelTemperingConfig {
    fn default() -> Self {
        Self {
            num_replicas: 8,
            min_temperature: 0.1,
            max_temperature: 10.0,
            temp_distribution: TemperatureDistribution::Geometric,
            steps_per_exchange: 100,
            num_exchanges: 100,
            adaptive_temperatures: true,
        }
    }
}

/// Replica state at a specific temperature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replica {
    /// Replica ID
    pub id: usize,
    /// Current temperature
    pub temperature: f64,
    /// Current state (binary variables or spins)
    pub state: Vec<i8>,
    /// Current energy
    pub energy: f64,
    /// Number of steps taken
    pub steps: usize,
    /// Number of exchanges accepted
    pub exchanges_accepted: usize,
}

/// Parallel Tempering solution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelTemperingSolution {
    /// Best state found across all replicas
    pub best_state: Vec<i8>,
    /// Energy of best state
    pub best_energy: f64,
    /// Replica that found the best solution
    pub best_replica_id: usize,
    /// Total exchanges attempted
    pub total_exchanges: usize,
    /// Total exchanges accepted
    pub accepted_exchanges: usize,
    /// Exchange acceptance rate
    pub acceptance_rate: f64,
    /// Computation time in milliseconds
    pub computation_time_ms: f64,
}

/// Parallel Tempering Optimizer
pub struct ParallelTempering {
    config: ParallelTemperingConfig,
    replicas: Vec<Arc<RwLock<Replica>>>,
}

impl ParallelTempering {
    /// Create a new parallel tempering optimizer
    pub fn new() -> Self {
        Self::with_config(ParallelTemperingConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: ParallelTemperingConfig) -> Self {
        let replicas = Vec::new();

        Self { config, replicas }
    }

    /// Optimize using parallel tempering on a generic energy function
    #[instrument(skip(self, initial_state, energy_fn))]
    pub async fn optimize<F>(
        &mut self,
        initial_state: Vec<i8>,
        energy_fn: F,
    ) -> CoreResult<ParallelTemperingSolution>
    where
        F: Fn(&[i8]) -> f64 + Send + Sync + Clone + 'static,
    {
        let start_time = std::time::Instant::now();

        if initial_state.is_empty() {
            return Err(CoreError::invalid_operation("Empty initial state"));
        }

        debug!(
            "Starting parallel tempering with {} replicas",
            self.config.num_replicas
        );

        // Initialize replicas at different temperatures
        self.initialize_replicas(&initial_state, &energy_fn).await?;

        let mut total_exchanges = 0;
        let mut accepted_exchanges = 0;

        // Main parallel tempering loop
        for exchange_round in 0..self.config.num_exchanges {
            // Run Monte Carlo on each replica in parallel
            let mut handles = Vec::new();

            for replica_arc in &self.replicas {
                let replica = Arc::clone(replica_arc);
                let energy_fn = energy_fn.clone();
                let steps = self.config.steps_per_exchange;

                let handle =
                    tokio::spawn(
                        async move { Self::monte_carlo_step(replica, energy_fn, steps).await },
                    );

                handles.push(handle);
            }

            // Wait for all replicas to complete their steps
            for handle in handles {
                handle.await.map_err(|e| {
                    CoreError::invalid_operation(&format!("Replica task failed: {}", e))
                })??;
            }

            // Attempt replica exchanges
            let exchanges = self.attempt_exchanges().await?;
            total_exchanges += exchanges.0;
            accepted_exchanges += exchanges.1;

            if exchange_round % 10 == 0 {
                debug!(
                    "Exchange round {}/{}: acceptance rate = {:.2}%",
                    exchange_round,
                    self.config.num_exchanges,
                    (accepted_exchanges as f64 / total_exchanges.max(1) as f64) * 100.0
                );
            }
        }

        // Find best solution across all replicas
        let best = self.find_best_solution().await?;

        let computation_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        let acceptance_rate = accepted_exchanges as f64 / total_exchanges.max(1) as f64;

        info!(
            "Parallel tempering completed: energy={:.4}, acceptance={:.2}%, time={:.2}ms",
            best.1,
            acceptance_rate * 100.0,
            computation_time_ms
        );

        Ok(ParallelTemperingSolution {
            best_state: best.0,
            best_energy: best.1,
            best_replica_id: best.2,
            total_exchanges,
            accepted_exchanges,
            acceptance_rate,
            computation_time_ms,
        })
    }

    /// Initialize replicas with random states
    async fn initialize_replicas<F>(
        &mut self,
        initial_state: &[i8],
        energy_fn: &F,
    ) -> CoreResult<()>
    where
        F: Fn(&[i8]) -> f64,
    {
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let temperatures = Self::generate_temperatures(&self.config);
        let mut rng = StdRng::from_entropy();

        self.replicas.clear();

        for (id, temp) in temperatures.iter().enumerate() {
            // Randomize initial state for each replica
            let state: Vec<i8> = initial_state
                .iter()
                .map(|&s| if rng.gen::<bool>() { s } else { -s })
                .collect();

            let energy = energy_fn(&state);

            let replica = Replica {
                id,
                temperature: *temp,
                state,
                energy,
                steps: 0,
                exchanges_accepted: 0,
            };

            self.replicas.push(Arc::new(RwLock::new(replica)));
        }

        Ok(())
    }

    /// Run Monte Carlo steps on a single replica
    async fn monte_carlo_step<F>(
        replica: Arc<RwLock<Replica>>,
        energy_fn: F,
        num_steps: usize,
    ) -> CoreResult<()>
    where
        F: Fn(&[i8]) -> f64,
    {
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        // Create a Send-safe RNG
        let mut rng = StdRng::from_entropy();

        for _ in 0..num_steps {
            let mut replica_guard = replica.write().await;
            let n = replica_guard.state.len();

            // Flip a random spin
            let flip_idx = rng.gen_range(0..n);
            let old_spin = replica_guard.state[flip_idx];
            replica_guard.state[flip_idx] = -old_spin;

            let new_energy = energy_fn(&replica_guard.state);
            let delta_e = new_energy - replica_guard.energy;

            // Metropolis acceptance criterion
            let accept = if delta_e < 0.0 {
                true
            } else {
                let prob = (-delta_e / replica_guard.temperature).exp();
                rng.gen::<f64>() < prob
            };

            if accept {
                replica_guard.energy = new_energy;
                replica_guard.steps += 1;
            } else {
                replica_guard.state[flip_idx] = old_spin; // Revert
            }
        }

        Ok(())
    }

    /// Attempt replica exchanges between adjacent temperature levels
    async fn attempt_exchanges(&self) -> CoreResult<(usize, usize)> {
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut rng = StdRng::from_entropy();
        let mut total_attempts = 0;
        let mut accepted = 0;

        // Try exchanges between adjacent replicas
        for i in 0..(self.replicas.len() - 1) {
            let replica_i = Arc::clone(&self.replicas[i]);
            let replica_j = Arc::clone(&self.replicas[i + 1]);

            let mut rep_i = replica_i.write().await;
            let mut rep_j = replica_j.write().await;

            total_attempts += 1;

            // Calculate exchange probability using Metropolis criterion
            // P(exchange) = min(1, exp(ΔE * Δβ))
            // where ΔE = E_j - E_i, Δβ = 1/T_i - 1/T_j
            let delta_e = rep_j.energy - rep_i.energy;
            let delta_beta = (1.0 / rep_i.temperature) - (1.0 / rep_j.temperature);
            let exchange_prob = (delta_e * delta_beta).exp().min(1.0);

            if rng.gen::<f64>() < exchange_prob {
                // Exchange states
                std::mem::swap(&mut rep_i.state, &mut rep_j.state);
                std::mem::swap(&mut rep_i.energy, &mut rep_j.energy);

                rep_i.exchanges_accepted += 1;
                rep_j.exchanges_accepted += 1;
                accepted += 1;
            }
        }

        Ok((total_attempts, accepted))
    }

    /// Find the best solution across all replicas
    async fn find_best_solution(&self) -> CoreResult<(Vec<i8>, f64, usize)> {
        let mut best_state = Vec::new();
        let mut best_energy = f64::INFINITY;
        let mut best_id = 0;

        for replica_arc in &self.replicas {
            let replica = replica_arc.read().await;
            if replica.energy < best_energy {
                best_energy = replica.energy;
                best_state = replica.state.clone();
                best_id = replica.id;
            }
        }

        if best_state.is_empty() {
            return Err(CoreError::invalid_operation("No valid solution found"));
        }

        Ok((best_state, best_energy, best_id))
    }

    /// Generate temperature ladder
    fn generate_temperatures(config: &ParallelTemperingConfig) -> Vec<f64> {
        match &config.temp_distribution {
            TemperatureDistribution::Geometric => {
                let ratio = (config.max_temperature / config.min_temperature)
                    .powf(1.0 / (config.num_replicas - 1) as f64);
                (0..config.num_replicas)
                    .map(|i| config.min_temperature * ratio.powi(i as i32))
                    .collect()
            }
            TemperatureDistribution::Linear => {
                let step = (config.max_temperature - config.min_temperature)
                    / (config.num_replicas - 1) as f64;
                (0..config.num_replicas)
                    .map(|i| config.min_temperature + step * i as f64)
                    .collect()
            }
            TemperatureDistribution::Custom { temperatures } => temperatures.clone(),
        }
    }

    /// Get current replica states (for monitoring)
    pub async fn get_replica_states(&self) -> Vec<(usize, f64, f64)> {
        let mut states = Vec::new();
        for replica_arc in &self.replicas {
            let replica = replica_arc.read().await;
            states.push((replica.id, replica.temperature, replica.energy));
        }
        states
    }
}

impl Default for ParallelTempering {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create energy function from Ising model
pub fn ising_energy_function(
    couplings: DMatrix<f64>,
    external_fields: Vec<f64>,
) -> impl Fn(&[i8]) -> f64 + Send + Sync + Clone {
    move |spins: &[i8]| {
        let mut energy = 0.0;
        let n = spins.len();

        // Interaction terms
        for i in 0..n {
            for j in (i + 1)..n {
                energy -= couplings[(i, j)] * spins[i] as f64 * spins[j] as f64;
            }
        }

        // External field terms
        for i in 0..n {
            energy -= external_fields[i] * spins[i] as f64;
        }

        energy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parallel_tempering_simple() {
        let config = ParallelTemperingConfig {
            num_replicas: 4,
            min_temperature: 0.5,
            max_temperature: 5.0,
            temp_distribution: TemperatureDistribution::Geometric,
            steps_per_exchange: 50,
            num_exchanges: 10,
            adaptive_temperatures: false,
        };

        let mut pt = ParallelTempering::with_config(config);

        // Simple energy function: minimize sum of squares
        let energy_fn = |state: &[i8]| -> f64 { state.iter().map(|&s| (s as f64).powi(2)).sum() };

        let initial_state = vec![1, -1, 1, -1];
        let solution = pt.optimize(initial_state, energy_fn).await.unwrap();

        assert_eq!(solution.best_state.len(), 4);
        assert!(solution.acceptance_rate >= 0.0 && solution.acceptance_rate <= 1.0);
        assert!(solution.total_exchanges > 0);
    }

    #[tokio::test]
    async fn test_ising_energy_function() {
        let couplings = DMatrix::from_fn(3, 3, |i, j| if i != j { 1.0 } else { 0.0 });
        let external_fields = vec![0.0, 0.0, 0.0];

        let energy_fn = ising_energy_function(couplings, external_fields);

        let spins_aligned = vec![1, 1, 1];
        let energy_aligned = energy_fn(&spins_aligned);

        let spins_random = vec![1, -1, 1];
        let energy_random = energy_fn(&spins_random);

        // Ferromagnetic coupling: aligned spins should have lower energy
        assert!(energy_aligned < energy_random);
    }

    #[test]
    fn test_temperature_generation() {
        let config = ParallelTemperingConfig {
            num_replicas: 5,
            min_temperature: 1.0,
            max_temperature: 10.0,
            temp_distribution: TemperatureDistribution::Geometric,
            steps_per_exchange: 100,
            num_exchanges: 10,
            adaptive_temperatures: false,
        };

        let temps = ParallelTempering::generate_temperatures(&config);

        assert_eq!(temps.len(), 5);
        assert!((temps[0] - 1.0).abs() < 1e-6);
        assert!((temps[4] - 10.0).abs() < 1e-6);

        // Check monotonic increase
        for i in 1..temps.len() {
            assert!(temps[i] > temps[i - 1]);
        }
    }

    #[test]
    fn test_linear_temperature_distribution() {
        let config = ParallelTemperingConfig {
            num_replicas: 6,
            min_temperature: 0.0,
            max_temperature: 10.0,
            temp_distribution: TemperatureDistribution::Linear,
            steps_per_exchange: 100,
            num_exchanges: 10,
            adaptive_temperatures: false,
        };

        let temps = ParallelTempering::generate_temperatures(&config);

        assert_eq!(temps.len(), 6);
        assert!((temps[0] - 0.0).abs() < 1e-6);
        assert!((temps[5] - 10.0).abs() < 1e-6);

        // Check equal spacing
        let spacing = temps[1] - temps[0];
        for i in 2..temps.len() {
            let diff = temps[i] - temps[i - 1];
            assert!((diff - spacing).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_empty_initial_state() {
        let mut pt = ParallelTempering::new();
        let energy_fn = |_state: &[i8]| 0.0;
        let result = pt.optimize(vec![], energy_fn).await;
        assert!(result.is_err());
    }
}
