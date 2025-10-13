//! # Neuromorphic Learning Demo
//!
//! This example demonstrates the complete neuromorphic network implementation
//! with Hebbian learning, synaptic plasticity, and adaptive query optimization.

use neuroquantum_core::synaptic::{
    ActivationFunction, ConnectionType, Neuron, Synapse, SynapticNetwork, SynapticNode,
};
use neuroquantum_core::error::CoreResult;

#[tokio::main]
async fn main() -> CoreResult<()> {
    // Initialize tracing for observability
    tracing_subscriber::fmt::init();

    println!("🧠 NeuroQuantumDB - Neuromorphic Learning Demo");
    println!("================================================\n");

    // Demo 1: Basic Neuron Activation Functions
    demo_activation_functions()?;

    // Demo 2: Synaptic Network with Hebbian Learning
    demo_hebbian_learning().await?;

    // Demo 3: Query Pattern Recognition and Adaptation
    demo_query_pattern_learning().await?;

    // Demo 4: Long-Term Potentiation (LTP) and Memory Consolidation
    demo_memory_consolidation().await?;

    // Demo 5: Adaptive Index Selection
    demo_adaptive_index_selection().await?;

    println!("\n✅ All neuromorphic learning demos completed successfully!");

    Ok(())
}

/// Demo 1: Activation Functions
fn demo_activation_functions() -> CoreResult<()> {
    println!("📊 Demo 1: Neuron Activation Functions");
    println!("---------------------------------------");

    let test_values = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
    let functions = vec![
        ActivationFunction::Sigmoid,
        ActivationFunction::ReLU,
        ActivationFunction::Tanh,
        ActivationFunction::LeakyReLU,
    ];

    for func in functions {
        println!("\n{:?}:", func);
        for &x in &test_values {
            let output = func.activate(x);
            let derivative = func.derivative(x);
            println!("  f({:.1}) = {:.4}, f'({:.1}) = {:.4}", x, output, x, derivative);
        }
    }

    println!("\n✓ Activation functions working correctly\n");
    Ok(())
}

/// Demo 2: Hebbian Learning - "Neurons that fire together, wire together"
async fn demo_hebbian_learning() -> CoreResult<()> {
    println!("🔗 Demo 2: Hebbian Learning");
    println!("---------------------------");

    let network = SynapticNetwork::new(1000, 0.3)?;

    // Create a simple network: Input -> Hidden -> Output
    println!("Creating neural network with 3 layers...");

    // Input layer (3 neurons)
    for i in 0..3 {
        let neuron = Neuron::new(i, ActivationFunction::Sigmoid);
        network.add_neuron(neuron)?;
    }

    // Hidden layer (4 neurons)
    for i in 3..7 {
        let neuron = Neuron::new(i, ActivationFunction::ReLU);
        network.add_neuron(neuron)?;
    }

    // Output layer (2 neurons)
    for i in 7..9 {
        let neuron = Neuron::new(i, ActivationFunction::Tanh);
        network.add_neuron(neuron)?;
    }

    // Create synapses between layers
    println!("Creating synaptic connections...");
    for pre in 0..3 {
        for post in 3..7 {
            let weight = (rand::random::<f32>() - 0.5) * 0.5;
            let synapse = Synapse::new(pre, post, weight);
            network.add_synapse(synapse)?;
        }
    }

    for pre in 3..7 {
        for post in 7..9 {
            let weight = (rand::random::<f32>() - 0.5) * 0.5;
            let synapse = Synapse::new(pre, post, weight);
            network.add_synapse(synapse)?;
        }
    }

    // Training with Hebbian learning
    println!("\nTraining with Hebbian learning (10 epochs)...");
    for epoch in 0..10 {
        // Input pattern: [1.0, 0.5, 0.8]
        let inputs = vec![1.0, 0.5, 0.8];
        let targets = vec![0.9, 0.1]; // Desired output

        // Forward propagation
        let outputs = network.forward_propagate(&inputs)?;

        // Apply Hebbian learning
        network.hebbian_update(&inputs, &targets)?;

        if epoch % 3 == 0 {
            println!(
                "  Epoch {}: Output = [{:.3}, {:.3}]",
                epoch,
                outputs.get(7).unwrap_or(&0.0),
                outputs.get(8).unwrap_or(&0.0)
            );
        }
    }

    println!("\n✓ Hebbian learning applied successfully\n");
    Ok(())
}

/// Demo 3: Query Pattern Recognition and Adaptation
async fn demo_query_pattern_learning() -> CoreResult<()> {
    println!("🎯 Demo 3: Query Pattern Recognition");
    println!("------------------------------------");

    let network = SynapticNetwork::new(1000, 0.4)?;

    // Add some neurons for pattern matching
    for i in 0..20 {
        let neuron = Neuron::new(i, ActivationFunction::Sigmoid);
        network.add_neuron(neuron)?;
    }

    // Create interconnected synapses
    for i in 0..15 {
        for j in (i + 1)..20 {
            if rand::random::<f32>() > 0.7 {
                let weight = rand::random::<f32>() * 0.5;
                let synapse = Synapse::new(i, j, weight);
                network.add_synapse(synapse)?;
            }
        }
    }

    // Simulate different query patterns
    let query_patterns = vec![
        ("SELECT * FROM users WHERE age > 25", vec![0.8, 0.6, 0.9, 0.7, 0.5]),
        ("SELECT name FROM products", vec![0.3, 0.9, 0.4, 0.8, 0.2]),
        ("SELECT * FROM users WHERE age > 25", vec![0.8, 0.6, 0.9, 0.7, 0.5]), // Repeated
    ];

    println!("Processing query patterns and learning...");
    for (i, (query, embedding)) in query_patterns.iter().enumerate() {
        // Simulate varying performance (better on repeated queries)
        let performance = if i == 2 { 0.85 } else { 0.60 };

        network.adapt_query_pattern(embedding, performance)?;

        println!(
            "  Query {}: '{}...' -> Performance: {:.2}",
            i + 1,
            &query[..30],
            performance
        );
    }

    println!("\n✓ Query patterns learned and adapted\n");
    Ok(())
}

/// Demo 4: Long-Term Potentiation and Memory Consolidation
async fn demo_memory_consolidation() -> CoreResult<()> {
    println!("💾 Demo 4: Long-Term Potentiation (LTP)");
    println!("---------------------------------------");

    let network = SynapticNetwork::new(1000, 0.3)?;

    // Create neurons
    for i in 0..10 {
        let neuron = Neuron::new(i, ActivationFunction::ReLU);
        network.add_neuron(neuron)?;
    }

    // Create synapses
    println!("Creating synaptic pathways...");
    let mut synapse_count = 0;
    for i in 0..8 {
        let synapse = Synapse::new(i, i + 1, 0.3);
        network.add_synapse(synapse)?;
        synapse_count += 1;
    }

    println!("  Created {} synapses", synapse_count);

    // Simulate correlated neural activity (co-activation)
    println!("\nSimulating correlated neural activity...");
    let activation_pairs = vec![
        (0, 1, 0.9),  // Strong correlation
        (1, 2, 0.85),
        (2, 3, 0.8),
        (4, 5, 0.4),  // Weak correlation
    ];

    network.apply_long_term_potentiation(&activation_pairs)?;
    println!("  Applied LTP to {} neuron pairs", activation_pairs.len());

    // Consolidate important memories
    println!("\nConsolidating important memory patterns...");

    // Add some query patterns to consolidate
    let important_patterns = vec![
        vec![0.9, 0.8, 0.7, 0.9, 0.6],
        vec![0.8, 0.9, 0.8, 0.7, 0.8],
    ];

    for (i, pattern) in important_patterns.iter().enumerate() {
        // High performance indicates importance
        network.adapt_query_pattern(pattern, 0.85)?;
        println!("  Pattern {} marked as important", i + 1);
    }

    network.consolidate_memory(0.7)?;
    println!("\n✓ Memory consolidation completed\n");
    Ok(())
}

/// Demo 5: Adaptive Index Selection
async fn demo_adaptive_index_selection() -> CoreResult<()> {
    println!("📈 Demo 5: Adaptive Index Selection");
    println!("-----------------------------------");

    let network = SynapticNetwork::new(1000, 0.4)?;

    // Setup network
    for i in 0..15 {
        let neuron = Neuron::new(i, ActivationFunction::Sigmoid);
        network.add_neuron(neuron)?;
    }

    println!("Training network on query patterns...");

    // Train on specific query patterns with high performance
    let query_embeddings = vec![
        (vec![0.9, 0.7, 0.8, 0.6, 0.9], 0.92, "Fast range scan"),
        (vec![0.3, 0.8, 0.4, 0.9, 0.3], 0.88, "Hash index lookup"),
        (vec![0.6, 0.5, 0.7, 0.5, 0.6], 0.45, "Full table scan (slow)"),
    ];

    for (i, (embedding, performance, description)) in query_embeddings.iter().enumerate() {
        network.adapt_query_pattern(embedding, *performance)?;

        let selected_index = network.select_adaptive_index(embedding)?;

        println!(
            "  Query {}: {} (perf: {:.2}) -> Index: {:?}",
            i + 1,
            description,
            performance,
            selected_index.unwrap_or_else(|| "None (learning)".to_string())
        );
    }

    println!("\n✓ Adaptive index selection working\n");
    Ok(())
}

