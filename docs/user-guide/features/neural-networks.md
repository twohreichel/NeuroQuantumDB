# Neural Networks

Neuromorphic learning with Hebbian plasticity.

## Concepts

| Term | Description |
|------|-------------|
| **Hebbian Learning** | "Neurons that fire together wire together" |
| **STDP** | Spike-Timing-Dependent Plasticity |
| **Lateral Inhibition** | Winner-takes-all competition |
| **Plasticity Matrix** | Adaptive weight reorganization |

## Usage

### Create Network

```bash
curl -X POST http://localhost:8080/api/v1/neural/create \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "pattern_detector",
    "layers": [10, 20, 10],
    "learning_rate": 0.01
  }'
```

### Train

```sql
NEURAL TRAIN pattern_detector
  ON training_data
  EPOCHS 100
  LEARNING_RATE 0.01;
```

### Predict

```sql
NEURAL PREDICT pattern_detector
  INPUT (0.5, 0.3, 0.8, 0.1, 0.9);
```

## Learning Rules

### Hebbian Update

```
Δw = η * pre * post
```

### Anti-Hebbian (Pruning)

```
Δw = -η * pre * post  (when correlation is negative)
```

### STDP Window

```
         Δw
          │    ╱
          │   ╱  LTP (pre before post)
          │  ╱
   ───────┼─────── Δt
          │╲
          │ ╲  LTD (post before pre)
          │  ╲
```

## Configuration

```toml
[neural]
learning_rate = 0.01
stdp_window_ms = 20
lateral_inhibition_radius = 3
plasticity_enabled = true
```
