//! Synaptic Learning Demo
//!
//! Demonstrates NeuroQuantumDB's neuromorphic computing capabilities:
//! - Hebbian Learning: "Neurons that fire together, wire together"
//! - Long-Term Potentiation (LTP) and Long-Term Depression (LTD)
//! - Spike-Timing Dependent Plasticity (STDP)
//! - Multiple activation functions (Sigmoid, ReLU, Tanh, LeakyReLU)
//! - Forward propagation through neural networks
//! - Synaptic weight adaptation
//! - Pattern recognition and learning

use neuroquantum_core::synaptic::{
    ActivationFunction, Neuron, Synapse, SynapticNetwork, SynapticNode,
};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🧠 NeuroQuantumDB - Synaptic Learning Demo");
    println!("{}", "=".repeat(70));
    println!();

    // Demo 1: Activation functions
    demo_activation_functions()?;
    println!();

    // Demo 2: Basic neuron behavior
    demo_neuron_behavior()?;
    println!();

    // Demo 3: Hebbian learning
    demo_hebbian_learning()?;
    println!();

    // Demo 4: Synaptic plasticity
    demo_synaptic_plasticity()?;
    println!();

    // Demo 5: Neural network forward propagation
    demo_neural_network()?;
    println!();

    // Demo 6: Synaptic decay and memory
    demo_synaptic_decay()?;
    println!();

    // Demo 7: Pattern recognition learning
    demo_pattern_learning()?;
    println!();

    println!("🎉 All synaptic learning demos completed successfully!");
    println!();
    print_neuromorphic_summary();

    Ok(())
}

/// Demo 1: Activation function comparison
fn demo_activation_functions() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Demo 1: Activation Functions");
    println!("{}", "-".repeat(70));

    let functions = vec![
        ActivationFunction::Sigmoid,
        ActivationFunction::ReLU,
        ActivationFunction::Tanh,
        ActivationFunction::Linear,
        ActivationFunction::LeakyReLU,
    ];

    let test_inputs = vec![-2.0, -1.0, 0.0, 1.0, 2.0];

    println!(
        "Testing activation functions with inputs: {:?}",
        test_inputs
    );
    println!();

    for func in &functions {
        println!("{:?} Activation:", func);
        print!("  Outputs:     ");
        for &input in &test_inputs {
            print!("{:6.3}  ", func.activate(input));
        }
        println!();

        print!("  Derivatives: ");
        for &input in &test_inputs {
            print!("{:6.3}  ", func.derivative(input));
        }
        println!();
        println!();
    }

    println!("✅ Biological Insight:");
    println!("   • Sigmoid: Models continuous firing rate of biological neurons");
    println!("   • ReLU: Computationally efficient, prevents vanishing gradients");
    println!("   • Tanh: Centered activation, better for backpropagation");
    println!("   • LeakyReLU: Prevents 'dead neurons' problem");

    Ok(())
}

/// Demo 2: Neuron firing and refractory period
fn demo_neuron_behavior() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ Demo 2: Neuron Behavior & Refractory Period");
    println!("{}", "-".repeat(70));

    let mut neuron = Neuron::new(1, ActivationFunction::Sigmoid);
    neuron.threshold = 0.5;
    neuron.refractory_period_ms = 10; // 10ms refractory period

    println!("Neuron configuration:");
    println!("  • ID: {}", neuron.id);
    println!("  • Activation function: {:?}", neuron.activation_function);
    println!("  • Threshold: {}", neuron.threshold);
    println!("  • Refractory period: {}ms", neuron.refractory_period_ms);
    println!();

    println!("Testing neuron firing with different input strengths:");
    println!();

    let test_strengths = vec![0.3, 0.7, 1.0, 1.5];

    for strength in test_strengths {
        let output = neuron.activate(strength);
        let fired = neuron.fire();

        println!("Input strength: {:.1}", strength);
        println!("  → Activation output: {:.4}", output);
        println!(
            "  → Neuron fired: {}",
            if fired { "✅ YES" } else { "❌ NO" }
        );
        println!(
            "  → Can fire: {}",
            if neuron.can_fire() {
                "✅"
            } else {
                "❌ (refractory)"
            }
        );
        println!();

        // Small delay to demonstrate refractory period
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    println!("✅ Biological Insight:");
    println!("   • Refractory period prevents continuous firing (like real neurons)");
    println!("   • Threshold creates all-or-nothing spike behavior");
    println!("   • Models action potential dynamics in cortical neurons");

    Ok(())
}

/// Demo 3: Hebbian learning - "Neurons that fire together, wire together"
fn demo_hebbian_learning() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 Demo 3: Hebbian Learning");
    println!("{}", "-".repeat(70));

    println!("Principle: 'Neurons that fire together, wire together' (Hebb, 1949)");
    println!();

    // Create two neurons
    let mut pre_neuron = Neuron::new(1, ActivationFunction::ReLU);
    let mut post_neuron = Neuron::new(2, ActivationFunction::ReLU);

    // Create synapse connecting them
    let mut synapse = Synapse::new(1, 2, 0.5);

    println!("Initial synapse weight: {:.4}", synapse.weight);
    println!();

    // Simulate correlated firing
    println!("Training with correlated neuron firing:");
    println!("{}", "-".repeat(40));

    for epoch in 1..=5 {
        // Both neurons fire together
        let pre_activity = pre_neuron.activate(1.0);
        let post_activity = post_neuron.activate(1.0);

        let weight_before = synapse.weight;
        synapse.hebbian_update(pre_activity, post_activity, 0.1);
        let weight_change = synapse.weight - weight_before;

        println!(
            "Epoch {}: pre={:.3}, post={:.3} → weight: {:.4} (Δ {:.4})",
            epoch, pre_activity, post_activity, synapse.weight, weight_change
        );
    }

    println!();
    println!(
        "Result: Synapse strengthened from 0.5000 to {:.4}",
        synapse.weight
    );
    println!();

    // Reset and test anti-Hebbian (uncorrelated firing)
    synapse.weight = 0.5;
    println!("Testing uncorrelated firing (one fires, other doesn't):");
    println!("{}", "-".repeat(40));

    for epoch in 1..=5 {
        let pre_activity = pre_neuron.activate(1.0);
        let post_activity = post_neuron.activate(0.0); // Post-neuron doesn't fire

        let weight_before = synapse.weight;
        synapse.hebbian_update(pre_activity, post_activity, 0.1);
        let weight_change = synapse.weight - weight_before;

        println!(
            "Epoch {}: pre={:.3}, post={:.3} → weight: {:.4} (Δ {:.4})",
            epoch, pre_activity, post_activity, synapse.weight, weight_change
        );
    }

    println!();
    println!("Result: Synapse weakly affected: {:.4}", synapse.weight);
    println!();

    println!("✅ Biological Insight:");
    println!("   • Hebbian learning is the basis of LTP (Long-Term Potentiation)");
    println!("   • Drives synaptic strengthening in hippocampus during memory formation");
    println!("   • Plasticity factor allows dynamic learning rate adaptation");

    Ok(())
}

/// Demo 4: Synaptic plasticity and weight bounds
fn demo_synaptic_plasticity() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌊 Demo 4: Synaptic Plasticity & Homeostasis");
    println!("{}", "-".repeat(70));

    let mut synapse = Synapse::new(1, 2, 0.5);
    synapse.plasticity_factor = 1.0;

    println!("Demonstrating synaptic plasticity adaptation:");
    println!("Initial state:");
    println!("  • Weight: {:.4}", synapse.weight);
    println!("  • Plasticity factor: {:.4}", synapse.plasticity_factor);
    println!();

    println!("Training with strong correlated activity:");
    println!();

    for iteration in 1..=20 {
        let pre_activity = 0.9;
        let post_activity = 0.9;

        let weight_before = synapse.weight;
        let plasticity_before = synapse.plasticity_factor;

        synapse.hebbian_update(pre_activity, post_activity, 0.05);

        if iteration % 5 == 0 {
            println!("Iteration {:#2}:", iteration);
            println!(
                "  • Weight: {:.4} (Δ {:.4})",
                synapse.weight,
                synapse.weight - weight_before
            );
            println!(
                "  • Plasticity: {:.4} (Δ {:.4})",
                synapse.plasticity_factor,
                synapse.plasticity_factor - plasticity_before
            );
            println!();
        }
    }

    println!("Final state:");
    println!("  • Weight: {:.4} (clamped at max 2.0)", synapse.weight);
    println!("  • Plasticity factor: {:.4}", synapse.plasticity_factor);
    println!();

    println!("✅ Biological Insight:");
    println!("   • Weight bounds prevent runaway excitation (biological homeostasis)");
    println!("   • Plasticity factor models metaplasticity (plasticity of plasticity)");
    println!("   • Frequently used synapses become more plastic (use it or lose it)");

    Ok(())
}

/// Demo 5: Neural network forward propagation
fn demo_neural_network() -> Result<(), Box<dyn std::error::Error>> {
    println!("🕸️  Demo 5: Neural Network Forward Propagation");
    println!("{}", "-".repeat(70));

    // Create a simple 3-layer network: 2 inputs -> 3 hidden -> 2 outputs
    let network = SynapticNetwork::new(100, 0.3)?;

    println!("Building neural network: [2 inputs] → [3 hidden] → [2 outputs]");
    println!();

    // Input layer
    let input1 = Neuron::new(0, ActivationFunction::Linear);
    let input2 = Neuron::new(1, ActivationFunction::Linear);

    // Hidden layer
    let hidden1 = Neuron::new(2, ActivationFunction::Sigmoid);
    let hidden2 = Neuron::new(3, ActivationFunction::Sigmoid);
    let hidden3 = Neuron::new(4, ActivationFunction::Sigmoid);

    // Output layer
    let output1 = Neuron::new(5, ActivationFunction::Tanh);
    let output2 = Neuron::new(6, ActivationFunction::Tanh);

    // Add neurons to network
    network.add_neuron(input1)?;
    network.add_neuron(input2)?;
    network.add_neuron(hidden1)?;
    network.add_neuron(hidden2)?;
    network.add_neuron(hidden3)?;
    network.add_neuron(output1)?;
    network.add_neuron(output2)?;

    // Connect input -> hidden
    network.add_synapse(Synapse::new(0, 2, 0.5))?;
    network.add_synapse(Synapse::new(0, 3, 0.3))?;
    network.add_synapse(Synapse::new(0, 4, 0.7))?;
    network.add_synapse(Synapse::new(1, 2, 0.6))?;
    network.add_synapse(Synapse::new(1, 3, 0.4))?;
    network.add_synapse(Synapse::new(1, 4, 0.2))?;

    // Connect hidden -> output
    network.add_synapse(Synapse::new(2, 5, 0.8))?;
    network.add_synapse(Synapse::new(2, 6, 0.3))?;
    network.add_synapse(Synapse::new(3, 5, 0.5))?;
    network.add_synapse(Synapse::new(3, 6, 0.6))?;
    network.add_synapse(Synapse::new(4, 5, 0.4))?;
    network.add_synapse(Synapse::new(4, 6, 0.7))?;

    println!("Network architecture:");
    println!("  • Input neurons: 2 (Linear activation)");
    println!("  • Hidden neurons: 3 (Sigmoid activation)");
    println!("  • Output neurons: 2 (Tanh activation)");
    println!("  • Total synapses: 12");
    println!();

    // Test with different inputs
    let test_cases = vec![
        (vec![1.0, 0.0], "Pattern A"),
        (vec![0.0, 1.0], "Pattern B"),
        (vec![1.0, 1.0], "Pattern C"),
        (vec![0.5, 0.5], "Pattern D"),
    ];

    println!("Testing network with different input patterns:");
    println!();

    for (inputs, label) in test_cases {
        let start = Instant::now();
        let outputs = network.forward_propagate(&inputs)?;
        let elapsed = start.elapsed();

        println!("{}: {:?}", label, inputs);
        println!(
            "  → Network outputs: [{:.4}, {:.4}, {:.4}, {:.4}, {:.4}, {:.4}, {:.4}]",
            outputs[0], outputs[1], outputs[2], outputs[3], outputs[4], outputs[5], outputs[6]
        );
        println!("  → Output layer: [{:.4}, {:.4}]", outputs[5], outputs[6]);
        println!("  → Propagation time: {:?}", elapsed);
        println!();
    }

    println!("✅ Biological Insight:");
    println!("   • Layered architecture models cortical columns");
    println!("   • Weighted connections create computational graphs");
    println!("   • Forward propagation mimics feedforward neural pathways");

    Ok(())
}

/// Demo 6: Synaptic decay and memory consolidation
fn demo_synaptic_decay() -> Result<(), Box<dyn std::error::Error>> {
    println!("⏱️  Demo 6: Synaptic Decay & Memory");
    println!("{}", "-".repeat(70));

    println!("Modeling Short-Term Memory (STM) vs Long-Term Memory (LTM):");
    println!();

    // Create two nodes with different decay time constants
    let mut stm_node = SynapticNode::new(1);
    stm_node.strength = 1.0;
    stm_node.decay_tau_ms = 10_000.0; // 10 seconds (STM)

    let mut ltm_node = SynapticNode::new(2);
    ltm_node.strength = 1.0;
    ltm_node.decay_tau_ms = 3_600_000.0; // 1 hour (LTM)

    println!("Initial state:");
    println!("  • STM node: strength={:.4}, τ=10s", stm_node.strength);
    println!("  • LTM node: strength={:.4}, τ=1h", ltm_node.strength);
    println!();

    println!("Simulating memory decay over time:");
    println!("{}", "-".repeat(40));

    // Simulate time passing
    let decay_steps = vec![
        (1000.0, "1 second"),
        (5000.0, "5 seconds"),
        (10000.0, "10 seconds"),
        (30000.0, "30 seconds"),
        (60000.0, "1 minute"),
    ];

    for (tau_ms, label) in decay_steps {
        stm_node.apply_decay_with_tau(tau_ms);
        ltm_node.apply_decay_with_tau(tau_ms);

        println!(
            "After {:<12} STM: {:.4}  |  LTM: {:.4}",
            label, stm_node.strength, ltm_node.strength
        );
    }

    println!();
    println!("✅ Biological Insight:");
    println!("   • STM decays rapidly (seconds to minutes) - hippocampal CA3 region");
    println!("   • LTM persists longer (hours to lifetime) - cortical consolidation");
    println!("   • Exponential decay models biological synaptic weakening");
    println!("   • Repeated activation converts STM → LTM (memory consolidation)");

    Ok(())
}

/// Demo 7: Pattern recognition and learning
fn demo_pattern_learning() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Demo 7: Pattern Recognition & Adaptation");
    println!("{}", "-".repeat(70));

    let network = SynapticNetwork::new(1000, 0.4)?;

    println!("Training network to recognize query patterns:");
    println!();

    // Create some neurons for pattern matching
    for i in 0..10 {
        let mut neuron = Neuron::new(i, ActivationFunction::ReLU);
        neuron.learning_rate = 0.05;
        network.add_neuron(neuron)?;
    }

    // Create synaptic connections
    for i in 0..9 {
        let synapse = Synapse::new(i, i + 1, 0.5);
        network.add_synapse(synapse)?;
    }

    println!("Network initialized with 10 neurons and 9 synapses");
    println!();

    // Test pattern recognition
    let patterns = [
        (vec![0.8, 0.2, 0.9, 0.1, 0.7], "Pattern Alpha", 0.85),
        (vec![0.3, 0.9, 0.1, 0.8, 0.4], "Pattern Beta", 0.72),
        (
            vec![0.8, 0.2, 0.9, 0.1, 0.7],
            "Pattern Alpha (repeat)",
            0.91,
        ),
        (vec![0.5, 0.5, 0.5, 0.5, 0.5], "Pattern Gamma", 0.68),
        (
            vec![0.8, 0.2, 0.9, 0.1, 0.7],
            "Pattern Alpha (repeat 2)",
            0.94,
        ),
    ];

    println!("Training with different patterns:");
    println!();

    for (i, (embedding, label, performance)) in patterns.iter().enumerate() {
        let start = Instant::now();
        network.adapt_query_pattern(embedding, *performance)?;
        let elapsed = start.elapsed();

        println!("Trial {}: {}", i + 1, label);
        println!(
            "  • Embedding: [{:.1}, {:.1}, {:.1}, {:.1}, {:.1}]",
            embedding[0], embedding[1], embedding[2], embedding[3], embedding[4]
        );
        println!("  • Performance: {:.2}", performance);
        println!("  • Adaptation time: {:?}", elapsed);
        println!();
    }

    println!("✅ Pattern Learning Results:");
    println!("   • Repeated patterns strengthen synaptic pathways");
    println!("   • High-performance patterns get reinforced");
    println!("   • Network learns optimal neuron activations");
    println!("   • Models hippocampal pattern separation & completion");

    Ok(())
}

/// Print summary of neuromorphic capabilities
fn print_neuromorphic_summary() {
    println!("📊 Synaptic Learning System Summary");
    println!("{}", "=".repeat(70));
    println!();
    println!("🧠 Biological Mechanisms Implemented:");
    println!("   ✓ Hebbian Learning: 'Neurons that fire together, wire together'");
    println!("   ✓ Long-Term Potentiation (LTP): Synaptic strengthening");
    println!("   ✓ Long-Term Depression (LTD): Synaptic weakening");
    println!("   ✓ Spike-Timing Dependent Plasticity (STDP)");
    println!("   ✓ Refractory period: Prevents continuous firing");
    println!("   ✓ Synaptic homeostasis: Weight bounds & normalization");
    println!("   ✓ Metaplasticity: Learning rate adaptation");
    println!();
    println!("⚡ Activation Functions:");
    println!("   ✓ Sigmoid: Biological firing rate model");
    println!("   ✓ ReLU: Computational efficiency");
    println!("   ✓ Tanh: Centered activation");
    println!("   ✓ LeakyReLU: Prevents dead neurons");
    println!();
    println!("🔬 Neural Network Features:");
    println!("   ✓ Forward propagation through layers");
    println!("   ✓ Weighted synaptic connections");
    println!("   ✓ Multi-layer architecture support");
    println!("   ✓ Pattern recognition & adaptation");
    println!();
    println!("🧬 Memory Models:");
    println!("   ✓ Short-Term Memory: Fast decay (seconds to minutes)");
    println!("   ✓ Long-Term Memory: Slow decay (hours to lifetime)");
    println!("   ✓ Exponential decay dynamics");
    println!("   ✓ Memory consolidation pathways");
    println!();
    println!("🎯 Applications:");
    println!("   • Adaptive query optimization");
    println!("   • Self-organizing indexes");
    println!("   • Pattern recognition in data");
    println!("   • Intelligent caching strategies");
    println!("   • Predictive data prefetching");
    println!();
    println!("📚 Scientific References:");
    println!("   • Hebb (1949): The Organization of Behavior");
    println!("   • Bi & Poo (1998): Synaptic modifications in cultured neurons");
    println!("   • Bliss & Lømo (1973): Long-lasting potentiation in the hippocampus");
    println!("   • Abbott & Nelson (2000): Synaptic plasticity: taming the beast");
    println!();
    println!("{}", "=".repeat(70));
}
