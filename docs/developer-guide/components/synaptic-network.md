# Synaptic Network

## Overview

Bio-inspired neural network with Hebbian learning.

## Structure

```rust
pub struct SynapticNetwork {
    nodes: Vec<SynapticNode>,
    connections: Vec<SynapticConnection>,
    plasticity_matrix: PlasticityMatrix,
    neon_optimizer: Option<NeonOptimizer>,
}

pub struct SynapticNode {
    id: NodeId,
    activation: f64,
    threshold: f64,
}

pub struct SynapticConnection {
    source: NodeId,
    target: NodeId,
    weight: f64,
    delay: f64,
}
```

## Learning Rules

### Hebbian Learning

```rust
impl HebbianLearning {
    pub fn update_weight(&self, pre: f64, post: f64, weight: &mut f64) {
        *weight += self.learning_rate * pre * post;
    }
}
```

### Anti-Hebbian (Pruning)

```rust
impl AntiHebbianLearning {
    pub fn prune(&self, weight: &mut f64, activity: f64) {
        if activity < self.threshold {
            *weight *= self.decay_factor;
        }
    }
}
```

### STDP

```rust
impl STDP {
    pub fn update(&self, delta_t: f64, weight: &mut f64) {
        if delta_t > 0.0 {
            // LTP: pre before post
            *weight += self.a_plus * (-delta_t / self.tau_plus).exp();
        } else {
            // LTD: post before pre
            *weight -= self.a_minus * (delta_t / self.tau_minus).exp();
        }
    }
}
```

## NEON Optimization

ARM64 SIMD acceleration for weight updates:

```rust
impl NeonOptimizer {
    pub fn optimize_connections(&self, nodes: &mut [SynapticNode]) -> Result<()>;
    pub fn is_enabled(&self) -> bool;
}
```

## Usage

```rust
let mut network = SynapticNetwork::new(config);

// Add nodes
network.add_node(SynapticNode::new(0, 0.5));
network.add_node(SynapticNode::new(1, 0.3));

// Connect
network.connect(0, 1, 0.5)?;

// Train
network.train(&input_data, epochs)?;

// Optimize with NEON (if available)
network.optimize_connections_with_neon()?;
```
