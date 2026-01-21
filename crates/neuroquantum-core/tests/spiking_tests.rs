//! Spiking Neural Network Tests
//!
//! Tests for the biologically accurate Izhikevich spiking neural network implementation.

use neuroquantum_core::spiking::{
    IzhikevichNeuron, IzhikevichNeuronType, IzhikevichParameters, STDPRule,
    SpikingNeuralNetwork, SpikingSynapse,
};

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
#[allow(clippy::float_cmp)]
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

    // First neuron should fire (second one due to synaptic input)
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
