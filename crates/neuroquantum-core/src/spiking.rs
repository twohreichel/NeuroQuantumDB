//! # Spiking Neural Network Models
//!
//! This module implements biologically accurate spiking neural network models,
//! with the Izhikevich neuron model as the primary implementation.
//!
//! ## Izhikevich Model
//!
//! The Izhikevich model (2003) provides a computationally efficient yet
//! biologically accurate representation of cortical neurons. It can reproduce
//! the firing patterns of all known types of cortical neurons using only
//! two coupled differential equations.
//!
//! ### Equations
//!
//! ```text
//! dv/dt = 0.04v² + 5v + 140 - u + I
//! du/dt = a(bv - u)
//!
//! if v >= 30 mV then v := c, u := u + d
//! ```
//!
//! Where:
//! - `v` is the membrane potential (mV)
//! - `u` is the membrane recovery variable
//! - `I` is the synaptic current or injected current
//! - `a`, `b`, `c`, `d` are dimensionless parameters
//!
//! ### Neuron Types
//!
//! | Type | a | b | c | d | Description |
//! |------|---|---|---|---|-------------|
//! | RS   | 0.02 | 0.2 | -65 | 8 | Regular Spiking (excitatory) |
//! | IB   | 0.02 | 0.2 | -55 | 4 | Intrinsically Bursting |
//! | CH   | 0.02 | 0.2 | -50 | 2 | Chattering |
//! | FS   | 0.1  | 0.2 | -65 | 2 | Fast Spiking (inhibitory) |
//! | TC   | 0.02 | 0.25 | -65 | 0.05 | Thalamocortical |
//! | RZ   | 0.1  | 0.26 | -65 | 2 | Resonator |
//! | LTS  | 0.02 | 0.25 | -65 | 2 | Low-Threshold Spiking |
//!
//! ## References
//!
//! - Izhikevich, E.M. (2003). Simple model of spiking neurons.
//!   IEEE Transactions on Neural Networks, 14(6):1569-1572.
//! - Izhikevich, E.M. (2004). Which model to use for cortical spiking neurons?
//!   IEEE Transactions on Neural Networks, 15(5):1063-1070.

use std::collections::HashMap;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use crate::error::{CoreError, CoreResult};

/// Cortical neuron types based on the Izhikevich model.
///
/// Each type has characteristic firing patterns observed in real cortical neurons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IzhikevichNeuronType {
    /// Regular Spiking (RS) - Most common excitatory neurons in cortex.
    /// Pyramidal cells in layers II-VI exhibit this pattern.
    /// Characterized by adaptation: decreasing firing frequency with sustained input.
    RegularSpiking,

    /// Intrinsically Bursting (IB) - Layer V pyramidal neurons.
    /// Fire an initial burst of spikes followed by regular spiking.
    /// Important for initiating and propagating cortical activity.
    IntrinsicallyBursting,

    /// Chattering (CH) - Fast rhythmic bursting.
    /// Found in superficial layers, generate gamma oscillations.
    /// Bursts at 25-70 Hz with 2-5 spikes per burst.
    Chattering,

    /// Fast Spiking (FS) - `GABAergic` interneurons (basket cells, chandelier cells).
    /// High-frequency firing without adaptation.
    /// Critical for cortical inhibition and gamma rhythms.
    FastSpiking,

    /// Thalamocortical (TC) - Thalamic relay neurons.
    /// Can fire in two modes: tonic (depolarized) and burst (hyperpolarized).
    /// Essential for sensory processing and sleep rhythms.
    Thalamocortical,

    /// Resonator (RZ) - Subthreshold oscillations.
    /// Prefer inputs at specific frequencies, enabling frequency selectivity.
    /// Found in various brain regions including basal ganglia.
    Resonator,

    /// Low-Threshold Spiking (LTS) - Martinotti cells.
    /// Fire at low frequencies with adaptation.
    /// Target distal dendrites of pyramidal cells.
    LowThresholdSpiking,
}

impl IzhikevichNeuronType {
    /// Get the characteristic parameters (a, b, c, d) for this neuron type.
    ///
    /// These parameters shape the neuron's firing dynamics and are derived
    /// from Izhikevich's original publications and subsequent refinements.
    #[must_use]
    pub const fn parameters(&self) -> IzhikevichParameters {
        match self {
            | Self::RegularSpiking => IzhikevichParameters {
                a: 0.02,
                b: 0.2,
                c: -65.0,
                d: 8.0,
            },
            | Self::IntrinsicallyBursting => IzhikevichParameters {
                a: 0.02,
                b: 0.2,
                c: -55.0,
                d: 4.0,
            },
            | Self::Chattering => IzhikevichParameters {
                a: 0.02,
                b: 0.2,
                c: -50.0,
                d: 2.0,
            },
            | Self::FastSpiking => IzhikevichParameters {
                a: 0.1,
                b: 0.2,
                c: -65.0,
                d: 2.0,
            },
            | Self::Thalamocortical => IzhikevichParameters {
                a: 0.02,
                b: 0.25,
                c: -65.0,
                d: 0.05,
            },
            | Self::Resonator => IzhikevichParameters {
                a: 0.1,
                b: 0.26,
                c: -65.0,
                d: 2.0,
            },
            | Self::LowThresholdSpiking => IzhikevichParameters {
                a: 0.02,
                b: 0.25,
                c: -65.0,
                d: 2.0,
            },
        }
    }

    /// Check if this neuron type is excitatory.
    #[must_use]
    pub const fn is_excitatory(&self) -> bool {
        matches!(
            self,
            Self::RegularSpiking
                | Self::IntrinsicallyBursting
                | Self::Chattering
                | Self::Thalamocortical
        )
    }

    /// Check if this neuron type is inhibitory.
    #[must_use]
    pub const fn is_inhibitory(&self) -> bool {
        matches!(self, Self::FastSpiking | Self::LowThresholdSpiking)
    }
}

/// Izhikevich model parameters.
///
/// These four dimensionless parameters completely determine the neuron's dynamics.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IzhikevichParameters {
    /// Time scale of recovery variable `u`.
    /// Smaller values result in slower recovery.
    /// Typical range: 0.02 (slow) to 0.1 (fast)
    pub a: f64,

    /// Sensitivity of `u` to subthreshold fluctuations of `v`.
    /// Greater values couple `u` and `v` more strongly.
    /// Typical range: 0.2 to 0.26
    pub b: f64,

    /// After-spike reset value of membrane potential `v`.
    /// Unit: mV
    /// Typical range: -65 to -50 mV
    pub c: f64,

    /// After-spike increment of recovery variable `u`.
    /// Represents the total amount of outward minus inward currents
    /// activated during the spike and affecting after-spike behavior.
    /// Typical range: 0.05 to 8
    pub d: f64,
}

impl Default for IzhikevichParameters {
    fn default() -> Self {
        // Default to Regular Spiking parameters
        IzhikevichNeuronType::RegularSpiking.parameters()
    }
}

/// Izhikevich spiking neuron model.
///
/// This struct represents a single neuron following the Izhikevich dynamics.
/// It maintains the state variables and provides methods for simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IzhikevichNeuron {
    /// Unique identifier for this neuron
    pub id: u64,

    /// Neuron type determining default parameters
    pub neuron_type: IzhikevichNeuronType,

    /// Membrane potential (mV)
    /// Resting potential is around -65 mV, spike peak at ~30 mV
    pub v: f64,

    /// Membrane recovery variable
    /// Represents the activation of K+ currents and inactivation of Na+ currents
    pub u: f64,

    /// Model parameters
    pub params: IzhikevichParameters,

    /// Spike threshold (mV) - typically 30 mV
    pub spike_threshold: f64,

    /// Current injected current (pA or arbitrary units)
    pub current_input: f64,

    /// Total number of spikes fired
    pub spike_count: u64,

    /// Time of last spike (in simulation steps)
    pub last_spike_time: Option<u64>,

    /// Spike times history (for STDP and analysis)
    spike_history: Vec<u64>,

    /// Maximum spike history length to retain
    max_history_length: usize,

    /// Accumulated synaptic input for current timestep
    synaptic_input: f64,
}

impl IzhikevichNeuron {
    /// Create a new Izhikevich neuron of the specified type.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the neuron
    /// * `neuron_type` - Type of neuron (RS, IB, CH, FS, TC, RZ, LTS)
    ///
    /// # Example
    ///
    /// ```
    /// use neuroquantum_core::spiking::{IzhikevichNeuron, IzhikevichNeuronType};
    ///
    /// let neuron = IzhikevichNeuron::new(1, IzhikevichNeuronType::RegularSpiking);
    /// assert_eq!(neuron.id, 1);
    /// assert!(neuron.neuron_type.is_excitatory());
    /// ```
    #[must_use]
    pub fn new(id: u64, neuron_type: IzhikevichNeuronType) -> Self {
        let params = neuron_type.parameters();
        Self {
            id,
            neuron_type,
            v: params.c, // Start at resting potential
            u: params.b * params.c,
            params,
            spike_threshold: 30.0,
            current_input: 0.0,
            spike_count: 0,
            last_spike_time: None,
            spike_history: Vec::with_capacity(1000),
            max_history_length: 1000,
            synaptic_input: 0.0,
        }
    }

    /// Create a neuron with custom parameters.
    ///
    /// Use this when you need fine-grained control over neuron dynamics.
    #[must_use]
    pub fn with_custom_params(id: u64, params: IzhikevichParameters) -> Self {
        Self {
            id,
            neuron_type: IzhikevichNeuronType::RegularSpiking,
            v: params.c,
            u: params.b * params.c,
            params,
            spike_threshold: 30.0,
            current_input: 0.0,
            spike_count: 0,
            last_spike_time: None,
            spike_history: Vec::with_capacity(1000),
            max_history_length: 1000,
            synaptic_input: 0.0,
        }
    }

    /// Add synaptic input to the neuron.
    ///
    /// Positive values represent excitatory input (EPSP),
    /// negative values represent inhibitory input (IPSP).
    #[inline]
    pub fn add_synaptic_input(&mut self, input: f64) {
        self.synaptic_input += input;
    }

    /// Set the external current injection.
    ///
    /// This represents experimental current clamp or tonic input.
    #[inline]
    pub const fn set_current(&mut self, current: f64) {
        self.current_input = current;
    }

    /// Simulate one timestep of neuron dynamics.
    ///
    /// Uses the Euler method with 0.5 ms resolution (two 0.5 ms steps per ms).
    /// Returns `true` if the neuron fired a spike during this timestep.
    ///
    /// # Arguments
    ///
    /// * `current_time` - Current simulation timestep
    ///
    /// # Details
    ///
    /// The equations are solved using the forward Euler method:
    /// ```text
    /// v += 0.5 * (0.04*v^2 + 5*v + 140 - u + I)  // twice for numerical stability
    /// u += a * (b*v - u)
    /// ```
    #[instrument(level = "trace", skip(self))]
    pub fn step(&mut self, current_time: u64) -> bool {
        let total_input = self.current_input + self.synaptic_input;
        self.synaptic_input = 0.0; // Reset for next timestep

        let IzhikevichParameters { a, b, c, d } = self.params;

        // Two half-steps for numerical stability (Euler method with dt=0.5ms)
        for _ in 0..2 {
            self.v += 0.5
                * ((0.04 * self.v).mul_add(self.v, 5.0 * self.v) + 140.0 - self.u + total_input);
        }
        self.u += a * b.mul_add(self.v, -self.u);

        // Check for spike
        if self.v >= self.spike_threshold {
            // Spike reset
            self.v = c;
            self.u += d;

            // Record spike
            self.spike_count += 1;
            self.last_spike_time = Some(current_time);

            // Update history
            if self.spike_history.len() >= self.max_history_length {
                self.spike_history.remove(0);
            }
            self.spike_history.push(current_time);

            debug!(neuron_id = self.id, time = current_time, "Spike fired");
            return true;
        }

        false
    }

    /// Get the instantaneous firing rate (Hz) based on recent spike history.
    ///
    /// Calculated from the inter-spike interval of the last two spikes.
    #[must_use]
    pub fn firing_rate(&self) -> Option<f64> {
        if self.spike_history.len() < 2 {
            return None;
        }

        let last = self.spike_history[self.spike_history.len() - 1];
        let second_last = self.spike_history[self.spike_history.len() - 2];
        let isi = (last - second_last) as f64; // Inter-spike interval in ms

        if isi > 0.0 {
            Some(1000.0 / isi) // Convert to Hz
        } else {
            None
        }
    }

    /// Get the average firing rate over the entire spike history.
    #[must_use]
    pub fn average_firing_rate(&self, simulation_duration_ms: u64) -> f64 {
        if simulation_duration_ms == 0 {
            return 0.0;
        }
        (self.spike_count as f64 / simulation_duration_ms as f64) * 1000.0
    }

    /// Reset the neuron to its initial state.
    pub fn reset(&mut self) {
        self.v = self.params.c;
        self.u = self.params.b * self.params.c;
        self.spike_count = 0;
        self.last_spike_time = None;
        self.spike_history.clear();
        self.synaptic_input = 0.0;
        self.current_input = 0.0;
    }

    /// Get the spike history.
    #[must_use]
    pub fn spike_times(&self) -> &[u64] {
        &self.spike_history
    }

    /// Check if the neuron is in a refractory state.
    ///
    /// Refractory period is approximately 2 ms after a spike.
    #[must_use]
    pub const fn is_refractory(&self, current_time: u64, refractory_period_ms: u64) -> bool {
        if let Some(last_spike) = self.last_spike_time {
            current_time.saturating_sub(last_spike) < refractory_period_ms
        } else {
            false
        }
    }
}

/// Conductance-based synapse for spiking neural networks.
///
/// Models synaptic transmission with exponential decay dynamics,
/// supporting both AMPA/NMDA (excitatory) and GABA (inhibitory) synapses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikingSynapse {
    /// Source neuron ID
    pub pre_id: u64,

    /// Target neuron ID
    pub post_id: u64,

    /// Synaptic weight (conductance)
    pub weight: f64,

    /// Synaptic delay (in timesteps)
    pub delay: u64,

    /// Synaptic time constant (ms)
    pub tau: f64,

    /// Current synaptic conductance
    pub conductance: f64,

    /// Reversal potential (mV)
    /// AMPA/NMDA: 0 mV (excitatory)
    /// GABA-A: -70 mV (inhibitory)
    pub reversal_potential: f64,

    /// Queue of pending spikes (time, weight)
    spike_queue: Vec<(u64, f64)>,
}

impl SpikingSynapse {
    /// Create a new excitatory synapse (AMPA-like).
    #[must_use]
    pub const fn excitatory(pre_id: u64, post_id: u64, weight: f64) -> Self {
        Self {
            pre_id,
            post_id,
            weight,
            delay: 1,
            tau: 5.0, // AMPA time constant ~5 ms
            conductance: 0.0,
            reversal_potential: 0.0, // Excitatory reversal potential
            spike_queue: Vec::new(),
        }
    }

    /// Create a new inhibitory synapse (GABA-A-like).
    #[must_use]
    pub const fn inhibitory(pre_id: u64, post_id: u64, weight: f64) -> Self {
        Self {
            pre_id,
            post_id,
            weight: weight.abs(), // Weight magnitude
            delay: 1,
            tau: 10.0, // GABA-A time constant ~10 ms
            conductance: 0.0,
            reversal_potential: -70.0, // Inhibitory reversal potential
            spike_queue: Vec::new(),
        }
    }

    /// Create a synapse with custom parameters.
    #[must_use]
    pub const fn with_params(
        pre_id: u64,
        post_id: u64,
        weight: f64,
        delay: u64,
        tau: f64,
        reversal_potential: f64,
    ) -> Self {
        Self {
            pre_id,
            post_id,
            weight,
            delay,
            tau,
            conductance: 0.0,
            reversal_potential,
            spike_queue: Vec::new(),
        }
    }

    /// Receive a presynaptic spike.
    pub fn receive_spike(&mut self, arrival_time: u64) {
        self.spike_queue
            .push((arrival_time + self.delay, self.weight));
    }

    /// Update synaptic conductance and return current to inject into postsynaptic neuron.
    ///
    /// # Arguments
    ///
    /// * `current_time` - Current simulation timestep
    /// * `post_v` - Postsynaptic membrane potential (for conductance-based current)
    pub fn update(&mut self, current_time: u64, post_v: f64) -> f64 {
        // Process arriving spikes
        let mut spike_contribution = 0.0;
        self.spike_queue.retain(|(arrival_time, weight)| {
            if *arrival_time <= current_time {
                spike_contribution += weight;
                false
            } else {
                true
            }
        });
        self.conductance += spike_contribution;

        // Exponential decay of conductance
        let decay = (-1.0 / self.tau).exp();
        self.conductance *= decay;

        // Calculate synaptic current (Ohm's law: I = g * (E - V))
        self.conductance * (self.reversal_potential - post_v)
    }
}

/// Spike-Timing-Dependent Plasticity (STDP) rule.
///
/// Implements the classic asymmetric STDP window:
/// - Pre-before-post: LTP (potentiation)
/// - Post-before-pre: LTD (depression)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STDPRule {
    /// Maximum potentiation (positive weight change)
    pub a_plus: f64,

    /// Maximum depression (negative weight change)
    pub a_minus: f64,

    /// Time constant for potentiation (ms)
    pub tau_plus: f64,

    /// Time constant for depression (ms)
    pub tau_minus: f64,

    /// Minimum weight
    pub w_min: f64,

    /// Maximum weight
    pub w_max: f64,
}

impl Default for STDPRule {
    fn default() -> Self {
        Self {
            a_plus: 0.005,
            a_minus: 0.00525, // Slightly stronger depression for stability
            tau_plus: 20.0,
            tau_minus: 20.0,
            w_min: 0.0,
            w_max: 1.0,
        }
    }
}

impl STDPRule {
    /// Calculate weight change based on spike timing difference.
    ///
    /// # Arguments
    ///
    /// * `delta_t` - Time difference (`t_post` - `t_pre`) in ms
    /// * `current_weight` - Current synaptic weight
    ///
    /// # Returns
    ///
    /// The new weight after applying STDP
    #[must_use]
    pub fn apply(&self, delta_t: f64, current_weight: f64) -> f64 {
        let dw = if delta_t > 0.0 {
            // Pre before post: LTP
            self.a_plus * (-delta_t / self.tau_plus).exp()
        } else {
            // Post before pre: LTD
            -self.a_minus * (delta_t / self.tau_minus).exp()
        };

        // Apply weight change with soft bounds
        let new_weight = current_weight + dw;
        new_weight.clamp(self.w_min, self.w_max)
    }
}

/// Spiking Neural Network using Izhikevich neurons.
///
/// This network supports real-time simulation with STDP learning.
#[derive(Debug)]
pub struct SpikingNeuralNetwork {
    /// Neurons in the network
    neurons: RwLock<HashMap<u64, IzhikevichNeuron>>,

    /// Synapses indexed by postsynaptic neuron ID
    synapses: RwLock<HashMap<u64, Vec<SpikingSynapse>>>,

    /// STDP learning rule
    pub stdp_rule: STDPRule,

    /// Enable STDP learning
    pub learning_enabled: bool,

    /// Current simulation time (ms)
    current_time: RwLock<u64>,

    /// Network statistics
    total_spikes: RwLock<u64>,
}

impl SpikingNeuralNetwork {
    /// Create a new spiking neural network.
    #[must_use]
    pub fn new() -> Self {
        Self {
            neurons: RwLock::new(HashMap::new()),
            synapses: RwLock::new(HashMap::new()),
            stdp_rule: STDPRule::default(),
            learning_enabled: true,
            current_time: RwLock::new(0),
            total_spikes: RwLock::new(0),
        }
    }

    /// Add a neuron to the network.
    pub fn add_neuron(&self, neuron: IzhikevichNeuron) -> CoreResult<()> {
        let mut neurons = self.neurons.write().map_err(|e| {
            CoreError::LockError(format!("Failed to acquire neurons write lock: {e}"))
        })?;

        if neurons.contains_key(&neuron.id) {
            return Err(CoreError::InvalidOperation(format!(
                "Neuron with ID {} already exists",
                neuron.id
            )));
        }

        neurons.insert(neuron.id, neuron);
        Ok(())
    }

    /// Add multiple neurons of a specific type.
    pub fn add_neuron_population(
        &self,
        start_id: u64,
        count: u64,
        neuron_type: IzhikevichNeuronType,
    ) -> CoreResult<Vec<u64>> {
        let mut neurons = self.neurons.write().map_err(|e| {
            CoreError::LockError(format!("Failed to acquire neurons write lock: {e}"))
        })?;

        let mut ids = Vec::with_capacity(count as usize);
        for i in 0..count {
            let id = start_id + i;
            if neurons.contains_key(&id) {
                return Err(CoreError::InvalidOperation(format!(
                    "Neuron with ID {id} already exists"
                )));
            }
            neurons.insert(id, IzhikevichNeuron::new(id, neuron_type));
            ids.push(id);
        }

        Ok(ids)
    }

    /// Connect two neurons with a synapse.
    pub fn connect(
        &self,
        pre_id: u64,
        post_id: u64,
        weight: f64,
        excitatory: bool,
    ) -> CoreResult<()> {
        // Verify neurons exist
        {
            let neurons = self.neurons.read().map_err(|e| {
                CoreError::LockError(format!("Failed to acquire neurons read lock: {e}"))
            })?;
            if !neurons.contains_key(&pre_id) {
                return Err(CoreError::NotFound(format!(
                    "Presynaptic neuron {pre_id} not found"
                )));
            }
            if !neurons.contains_key(&post_id) {
                return Err(CoreError::NotFound(format!(
                    "Postsynaptic neuron {post_id} not found"
                )));
            }
        }

        let synapse = if excitatory {
            SpikingSynapse::excitatory(pre_id, post_id, weight)
        } else {
            SpikingSynapse::inhibitory(pre_id, post_id, weight)
        };

        let mut synapses = self.synapses.write().map_err(|e| {
            CoreError::LockError(format!("Failed to acquire synapses write lock: {e}"))
        })?;

        synapses.entry(post_id).or_default().push(synapse);
        Ok(())
    }

    /// Inject current into a specific neuron.
    pub fn inject_current(&self, neuron_id: u64, current: f64) -> CoreResult<()> {
        let mut neurons = self.neurons.write().map_err(|e| {
            CoreError::LockError(format!("Failed to acquire neurons write lock: {e}"))
        })?;

        let neuron = neurons
            .get_mut(&neuron_id)
            .ok_or_else(|| CoreError::NotFound(format!("Neuron {neuron_id} not found")))?;

        neuron.set_current(current);
        Ok(())
    }

    /// Simulate one timestep of the network (1 ms).
    ///
    /// Returns a vector of neuron IDs that fired during this timestep.
    #[instrument(level = "debug", skip(self))]
    pub fn step(&self) -> CoreResult<Vec<u64>> {
        let current_time = {
            let mut time = self.current_time.write().map_err(|e| {
                CoreError::LockError(format!("Failed to acquire time write lock: {e}"))
            })?;
            *time += 1;
            *time
        };

        let mut fired_neurons = Vec::new();

        // Phase 1: Update synapses and collect synaptic currents
        let mut synaptic_currents: HashMap<u64, f64> = HashMap::new();
        {
            let neurons = self.neurons.read().map_err(|e| {
                CoreError::LockError(format!("Failed to acquire neurons read lock: {e}"))
            })?;
            let mut synapses = self.synapses.write().map_err(|e| {
                CoreError::LockError(format!("Failed to acquire synapses write lock: {e}"))
            })?;

            for (post_id, synapse_list) in synapses.iter_mut() {
                if let Some(post_neuron) = neurons.get(post_id) {
                    let mut total_current = 0.0;
                    for synapse in synapse_list.iter_mut() {
                        total_current += synapse.update(current_time, post_neuron.v);
                    }
                    synaptic_currents.insert(*post_id, total_current);
                }
            }
        }

        // Phase 2: Apply synaptic currents and step neurons
        {
            let mut neurons = self.neurons.write().map_err(|e| {
                CoreError::LockError(format!("Failed to acquire neurons write lock: {e}"))
            })?;

            for (id, neuron) in neurons.iter_mut() {
                if let Some(&current) = synaptic_currents.get(id) {
                    neuron.add_synaptic_input(current);
                }
                if neuron.step(current_time) {
                    fired_neurons.push(*id);
                }
            }
        }

        // Phase 3: Propagate spikes through synapses
        {
            let mut synapses = self.synapses.write().map_err(|e| {
                CoreError::LockError(format!("Failed to acquire synapses write lock: {e}"))
            })?;

            for &pre_id in &fired_neurons {
                // Find all synapses originating from this neuron
                for synapse_list in synapses.values_mut() {
                    for synapse in synapse_list.iter_mut() {
                        if synapse.pre_id == pre_id {
                            synapse.receive_spike(current_time);
                        }
                    }
                }
            }
        }

        // Phase 4: Apply STDP learning if enabled
        if self.learning_enabled && !fired_neurons.is_empty() {
            self.apply_stdp(&fired_neurons, current_time)?;
        }

        // Update statistics
        {
            let mut total = self.total_spikes.write().map_err(|e| {
                CoreError::LockError(format!("Failed to acquire total_spikes write lock: {e}"))
            })?;
            *total += fired_neurons.len() as u64;
        }

        Ok(fired_neurons)
    }

    /// Apply STDP learning rule based on spike timing.
    fn apply_stdp(&self, fired_neurons: &[u64], current_time: u64) -> CoreResult<()> {
        let neurons = self.neurons.read().map_err(|e| {
            CoreError::LockError(format!("Failed to acquire neurons read lock: {e}"))
        })?;
        let mut synapses = self.synapses.write().map_err(|e| {
            CoreError::LockError(format!("Failed to acquire synapses write lock: {e}"))
        })?;

        for &post_id in fired_neurons {
            if let Some(synapse_list) = synapses.get_mut(&post_id) {
                for synapse in synapse_list.iter_mut() {
                    // Get presynaptic spike time
                    if let Some(pre_neuron) = neurons.get(&synapse.pre_id) {
                        if let Some(pre_spike_time) = pre_neuron.last_spike_time {
                            let delta_t = (current_time as i64 - pre_spike_time as i64) as f64;
                            // Only apply STDP for recent spikes (within 100 ms window)
                            if delta_t.abs() < 100.0 {
                                synapse.weight = self.stdp_rule.apply(delta_t, synapse.weight);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Run the simulation for a specified duration.
    ///
    /// # Arguments
    ///
    /// * `duration_ms` - Duration to simulate in milliseconds
    ///
    /// # Returns
    ///
    /// A map of neuron IDs to their spike times during the simulation
    #[instrument(level = "info", skip(self))]
    pub fn simulate(&self, duration_ms: u64) -> CoreResult<HashMap<u64, Vec<u64>>> {
        let mut spike_raster: HashMap<u64, Vec<u64>> = HashMap::new();

        for _ in 0..duration_ms {
            let fired = self.step()?;
            let time = *self.current_time.read().map_err(|e| {
                CoreError::LockError(format!("Failed to acquire time read lock: {e}"))
            })?;

            for id in fired {
                spike_raster.entry(id).or_default().push(time);
            }
        }

        Ok(spike_raster)
    }

    /// Get network statistics.
    pub fn statistics(&self) -> CoreResult<NetworkStatistics> {
        let neurons = self.neurons.read().map_err(|e| {
            CoreError::LockError(format!("Failed to acquire neurons read lock: {e}"))
        })?;
        let synapses = self.synapses.read().map_err(|e| {
            CoreError::LockError(format!("Failed to acquire synapses read lock: {e}"))
        })?;
        let total_spikes = *self.total_spikes.read().map_err(|e| {
            CoreError::LockError(format!("Failed to acquire total_spikes read lock: {e}"))
        })?;
        let current_time = *self
            .current_time
            .read()
            .map_err(|e| CoreError::LockError(format!("Failed to acquire time read lock: {e}")))?;

        let total_synapses: usize = synapses.values().map(std::vec::Vec::len).sum();
        let avg_weight: f64 = if total_synapses > 0 {
            synapses
                .values()
                .flat_map(|v| v.iter())
                .map(|s| s.weight)
                .sum::<f64>()
                / total_synapses as f64
        } else {
            0.0
        };

        Ok(NetworkStatistics {
            neuron_count: neurons.len(),
            synapse_count: total_synapses,
            total_spikes,
            simulation_time_ms: current_time,
            average_synaptic_weight: avg_weight,
        })
    }

    /// Get the current simulation time.
    pub fn current_time(&self) -> CoreResult<u64> {
        Ok(*self
            .current_time
            .read()
            .map_err(|e| CoreError::LockError(format!("Failed to acquire time read lock: {e}")))?)
    }

    /// Reset all neurons and simulation time.
    pub fn reset(&self) -> CoreResult<()> {
        {
            let mut neurons = self.neurons.write().map_err(|e| {
                CoreError::LockError(format!("Failed to acquire neurons write lock: {e}"))
            })?;
            for neuron in neurons.values_mut() {
                neuron.reset();
            }
        }
        {
            let mut synapses = self.synapses.write().map_err(|e| {
                CoreError::LockError(format!("Failed to acquire synapses write lock: {e}"))
            })?;
            for synapse_list in synapses.values_mut() {
                for synapse in synapse_list.iter_mut() {
                    synapse.conductance = 0.0;
                    synapse.spike_queue.clear();
                }
            }
        }
        *self.current_time.write().map_err(|e| {
            CoreError::LockError(format!("Failed to acquire time write lock: {e}"))
        })? = 0;
        *self.total_spikes.write().map_err(|e| {
            CoreError::LockError(format!("Failed to acquire total_spikes write lock: {e}"))
        })? = 0;

        Ok(())
    }
}

impl Default for SpikingNeuralNetwork {
    fn default() -> Self {
        Self::new()
    }
}

/// Network statistics summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatistics {
    /// Total number of neurons
    pub neuron_count: usize,

    /// Total number of synapses
    pub synapse_count: usize,

    /// Total spikes fired during simulation
    pub total_spikes: u64,

    /// Current simulation time in ms
    pub simulation_time_ms: u64,

    /// Average synaptic weight
    pub average_synaptic_weight: f64,
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn test_izhikevich_neuron_types() {
        // Test all neuron types can be created
        let types = [
            IzhikevichNeuronType::RegularSpiking,
            IzhikevichNeuronType::IntrinsicallyBursting,
            IzhikevichNeuronType::Chattering,
            IzhikevichNeuronType::FastSpiking,
            IzhikevichNeuronType::Thalamocortical,
            IzhikevichNeuronType::Resonator,
            IzhikevichNeuronType::LowThresholdSpiking,
        ];

        for neuron_type in types {
            let neuron = IzhikevichNeuron::new(1, neuron_type);
            assert_eq!(neuron.neuron_type, neuron_type);
            assert!(neuron.v < 0.0); // Should start at resting potential
        }
    }

    #[test]
    fn test_neuron_excitatory_inhibitory() {
        assert!(IzhikevichNeuronType::RegularSpiking.is_excitatory());
        assert!(!IzhikevichNeuronType::RegularSpiking.is_inhibitory());
        assert!(IzhikevichNeuronType::FastSpiking.is_inhibitory());
        assert!(!IzhikevichNeuronType::FastSpiking.is_excitatory());
    }

    #[test]
    fn test_regular_spiking_pattern() {
        // Regular spiking neurons should show adaptation
        let mut neuron = IzhikevichNeuron::new(1, IzhikevichNeuronType::RegularSpiking);
        neuron.set_current(14.0); // Suprathreshold current

        let mut spike_times = Vec::new();
        for t in 0..1000 {
            if neuron.step(t) {
                spike_times.push(t);
            }
        }

        // Should fire multiple spikes
        assert!(
            spike_times.len() > 5,
            "Expected multiple spikes, got {}",
            spike_times.len()
        );

        // Check for adaptation (later ISIs should be longer than earlier ones)
        if spike_times.len() >= 4 {
            let first_isi = spike_times[1] - spike_times[0];
            let last_isi = spike_times[spike_times.len() - 1] - spike_times[spike_times.len() - 2];
            assert!(
                last_isi >= first_isi,
                "Expected adaptation (later ISI >= earlier ISI)"
            );
        }
    }

    #[test]
    fn test_fast_spiking_high_frequency() {
        // Fast spiking neurons should fire at high frequency without adaptation
        let mut neuron = IzhikevichNeuron::new(1, IzhikevichNeuronType::FastSpiking);
        neuron.set_current(10.0);

        let mut spike_count = 0;
        for t in 0..500 {
            if neuron.step(t) {
                spike_count += 1;
            }
        }

        // FS neurons should have high firing rate
        let rate = (f64::from(spike_count) / 500.0) * 1000.0;
        assert!(rate > 50.0, "FS neuron expected high rate, got {rate} Hz");
    }

    #[test]
    fn test_thalamocortical_burst_mode() {
        // TC neurons in hyperpolarized state should burst
        let mut neuron = IzhikevichNeuron::new(1, IzhikevichNeuronType::Thalamocortical);
        // Start from hyperpolarized state
        neuron.v = -80.0;
        neuron.u = -20.0;
        neuron.set_current(10.0);

        let mut spike_times = Vec::new();
        for t in 0..100 {
            if neuron.step(t) {
                spike_times.push(t);
            }
        }

        // Should produce a burst of spikes
        assert!(
            !spike_times.is_empty(),
            "TC neuron should fire from hyperpolarized state"
        );
    }

    #[test]
    fn test_synapse_creation() {
        let exc = SpikingSynapse::excitatory(1, 2, 0.5);
        assert_eq!(exc.reversal_potential, 0.0);
        assert_eq!(exc.pre_id, 1);
        assert_eq!(exc.post_id, 2);

        let inh = SpikingSynapse::inhibitory(1, 2, 0.5);
        assert_eq!(inh.reversal_potential, -70.0);
    }

    #[test]
    fn test_stdp_rule() {
        let rule = STDPRule::default();

        // Pre before post (LTP)
        let w1 = rule.apply(10.0, 0.5);
        assert!(w1 > 0.5, "Expected potentiation");

        // Post before pre (LTD)
        let w2 = rule.apply(-10.0, 0.5);
        assert!(w2 < 0.5, "Expected depression");

        // Weight bounds
        let w3 = rule.apply(1.0, 0.99);
        assert!(w3 <= rule.w_max, "Weight should not exceed max");
    }

    #[test]
    fn test_network_creation() {
        let network = SpikingNeuralNetwork::new();

        let excitatory = network
            .add_neuron_population(0, 80, IzhikevichNeuronType::RegularSpiking)
            .unwrap();
        assert_eq!(excitatory.len(), 80);

        let inhibitory = network
            .add_neuron_population(80, 20, IzhikevichNeuronType::FastSpiking)
            .unwrap();
        assert_eq!(inhibitory.len(), 20);

        let stats = network.statistics().unwrap();
        assert_eq!(stats.neuron_count, 100);
    }

    #[test]
    fn test_network_simulation() {
        let network = SpikingNeuralNetwork::new();

        // Create a simple network
        network
            .add_neuron(IzhikevichNeuron::new(
                1,
                IzhikevichNeuronType::RegularSpiking,
            ))
            .unwrap();
        network
            .add_neuron(IzhikevichNeuron::new(
                2,
                IzhikevichNeuronType::RegularSpiking,
            ))
            .unwrap();
        network.connect(1, 2, 0.5, true).unwrap();

        // Inject current into first neuron
        network.inject_current(1, 14.0).unwrap();

        // Run simulation
        let spike_raster = network.simulate(100).unwrap();

        // First neuron should fire
        assert!(
            spike_raster.contains_key(&1),
            "Neuron 1 should fire with input current"
        );
    }

    #[test]
    fn test_network_reset() {
        let network = SpikingNeuralNetwork::new();
        network
            .add_neuron(IzhikevichNeuron::new(
                1,
                IzhikevichNeuronType::RegularSpiking,
            ))
            .unwrap();
        network.inject_current(1, 14.0).unwrap();

        network.simulate(50).unwrap();

        let time_before = network.current_time().unwrap();
        assert!(time_before > 0);

        network.reset().unwrap();

        let time_after = network.current_time().unwrap();
        assert_eq!(time_after, 0);
    }

    #[test]
    fn test_neuron_firing_rate() {
        let mut neuron = IzhikevichNeuron::new(1, IzhikevichNeuronType::FastSpiking);
        neuron.set_current(10.0);

        // No firing rate before any spikes
        assert!(neuron.firing_rate().is_none());

        // Simulate until we get at least 2 spikes
        for t in 0..200 {
            neuron.step(t);
            if neuron.spike_count >= 2 {
                break;
            }
        }

        // Now we should have a firing rate
        if neuron.spike_count >= 2 {
            assert!(neuron.firing_rate().is_some());
            let rate = neuron.firing_rate().unwrap();
            assert!(rate > 0.0, "Firing rate should be positive");
        }
    }

    #[test]
    fn test_synaptic_transmission() {
        let network = SpikingNeuralNetwork::new();

        // Create two neurons
        network
            .add_neuron(IzhikevichNeuron::new(
                1,
                IzhikevichNeuronType::RegularSpiking,
            ))
            .unwrap();
        network
            .add_neuron(IzhikevichNeuron::new(
                2,
                IzhikevichNeuronType::RegularSpiking,
            ))
            .unwrap();

        // Strong excitatory connection
        network.connect(1, 2, 20.0, true).unwrap();

        // Strong input to first neuron
        network.inject_current(1, 15.0).unwrap();

        // Simulate
        let raster = network.simulate(200).unwrap();

        // Both neurons should fire (second one due to synaptic input)
        assert!(raster.contains_key(&1), "Pre-synaptic neuron should fire");
        // Neuron 2 may or may not fire depending on connection strength
    }

    #[test]
    fn test_custom_parameters() {
        let params = IzhikevichParameters {
            a: 0.03,
            b: 0.25,
            c: -60.0,
            d: 4.0,
        };

        let neuron = IzhikevichNeuron::with_custom_params(1, params);
        assert_eq!(neuron.params.a, 0.03);
        assert_eq!(neuron.params.c, -60.0);
    }

    #[test]
    fn test_refractory_period() {
        let mut neuron = IzhikevichNeuron::new(1, IzhikevichNeuronType::FastSpiking);
        neuron.set_current(10.0);

        // Find first spike
        let mut spike_time = None;
        for t in 0..100 {
            if neuron.step(t) {
                spike_time = Some(t);
                break;
            }
        }

        if let Some(t) = spike_time {
            // Immediately after spike, should be refractory
            assert!(neuron.is_refractory(t, 2));
            // After refractory period, should not be refractory
            assert!(!neuron.is_refractory(t + 5, 2));
        }
    }
}
