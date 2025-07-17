
# 🤖 BLEEP AI Decision Module

The **BLEEP AI Decision Module** is a powerful real-time inference engine enabling blockchain-integrated AI-driven decision-making. It supports both ONNX-based predictive models and ensemble classification with aggregation strategies.

---

## 🧠 Key Capabilities

- **ONNX Inference**: Fast inference using compiled ONNX models with runtime safety.
- **Custom Prediction Types**: Supports classification, float vectors, and byte vectors.
- **Caching**: DashMap-based state caching with expiration to reduce recomputation.
- **Asynchronous Execution**: Tokio-powered asynchronous model registration and prediction.
- **Ensemble Aggregation**: Supports majority voting, averaging, and weighted strategies.
- **Timeout Resilience**: Limits blocking predictions with graceful fallbacks.

---

## 🔍 Core Components

### `BLEEPAIDecisionModule`
Manages AI models, performs predictions, and caches results.

```rust
pub struct BLEEPAIDecisionModule {
    models: HashMap<String, Arc<dyn AIModel>>,
    state_cache: DashMap<String, (PredictionResult, Instant)>,
}
```

#### Main Functions:
- `register_model(name, model)` — Registers a model by name.
- `predict(name, input)` — Predicts using registered model with cache support.

---

### `AIModel` Trait

```rust
pub trait AIModel: Send + Sync {
    fn predict(&self, input: &[f32]) -> Result<PredictionResult, BLEEPError>;
}
```

Allows dynamic registration of different model types.

---

### `ONNXPredictiveModel`

Loads and runs ONNX inference models.

```rust
let model = ONNXPredictiveModel::new("path_to_model.onnx")?;
let result = model.predict(&[0.5, 0.2, 0.7])?;
```

---

### `EnsemblePredictiveModel`

Combines predictions from multiple models using a strategy.

Supported strategies:
- `MajorityVote`
- `Average`
- `WeightedAverage(Vec<f32>)`

---

## 📦 Prediction Result Types

```rust
pub enum PredictionResult {
    ByteVec(Vec<u8>),
    FloatVec(Vec<f32>),
    Classification(String),
    Default,
}
```

---

## 🔒 Error Handling

Handles various AI-related errors through `BLEEPError` enum.

```rust
#[derive(Debug, Error)]
pub enum BLEEPError {
    InvalidInput,
    ModelNotFoundError,
    PredictionError,
    TimeoutError,
    ModelAlreadyRegistered,
    BlockchainIntegrationError,
}
```

---

## 🧪 MLModel for Classification

Includes test cases and model loading via ONNX.

```rust
let ml_model = MLModel::new("class_model", "models/example_model.onnx")?;
let label = ml_model.classify(&[0.1, 0.9, 0.0], &["Low", "Medium", "High"])?;
```

---

## ✅ Summary

BLEEP's AI Decision Module provides scalable, secure, and intelligent inference for real-time blockchain environments. Designed with modularity, extensibility, and robustness in mind.

