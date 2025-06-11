use onnxruntime::{
    environment::Environment,
    error::OrtError,
    session::{Session, SessionBuilder},
    tensor::{OrtOwnedTensor, TensorElementData},
};
use std::sync::Arc;
use thiserror::Error;
use serde::{Serialize, Deserialize};

// Define custom error types
#[derive(Debug, Error)]
pub enum MLError {
    #[error("Failed to load model")]
    ModelLoadError,
    #[error("Prediction failed")]
    PredictionError,
    #[error("Invalid input format")]
    InvalidInput,
}

// Flexible return type for ML predictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MLResult {
    FloatVec(Vec<f32>),
    Classification(String),
    Default,
}

// Machine Learning Model structure
pub struct MLModel {
    pub name: String,
    session: Arc<Session>, // ONNX session for running model inference
}

impl MLModel {
    /// Creates a new MLModel instance by loading a pre-trained ONNX model
    pub fn new(name: &str, model_path: &str) -> Result<Self, MLError> {
        let environment = Environment::builder()
            .with_name("BLEEP_ML_Environment")
            .build()
            .map_err(|_| MLError::ModelLoadError)?;

        let session = SessionBuilder::new(&environment)
            .map_err(|_| MLError::ModelLoadError)?
            .with_model_from_file(model_path)
            .map_err(|_| MLError::ModelLoadError)?;

        Ok(Self {
            name: name.to_string(),
            session: Arc::new(session),
        })
    }

    /// Predicts the output using the model for the given input
    pub fn predict(&self, input_data: &[f32]) -> Result<MLResult, MLError> {
        // Ensure input data is valid
        if input_data.is_empty() {
            return Err(MLError::InvalidInput);
        }

        // Create input tensor
        let input_tensor = vec![input_data.to_vec()]; // 1D tensor with input data

        // Run the inference
        let outputs: Vec<OrtOwnedTensor<f32, _>> = self
            .session
            .run(input_tensor)
            .map_err(|_| MLError::PredictionError)?;

        // Extract the prediction results
        let result = outputs
            .into_iter()
            .flat_map(|tensor| tensor.iter().cloned())
            .collect::<Vec<f32>>();

        Ok(MLResult::FloatVec(result))
    }

    /// Classifies the input using predefined class labels
    pub fn classify(&self, input_data: &[f32], class_labels: &[&str]) -> Result<MLResult, MLError> {
        let prediction = self.predict(input_data)?;

        if let MLResult::FloatVec(probabilities) = prediction {
            // Find the class with the highest probability
            if let Some((index, _)) = probabilities
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            {
                let class_label = class_labels.get(index).unwrap_or(&"Unknown").to_string();
                return Ok(MLResult::Classification(class_label));
            }
        }

        Ok(MLResult::Default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_prediction() {
        let model_path = "models/example_model.onnx"; // Replace with a real ONNX model path
        let model = MLModel::new("example_model", model_path).expect("Failed to load model");

        let input_data = vec![0.5, 0.8, 0.2];
        let prediction = model.predict(&input_data);

        assert!(prediction.is_ok(), "Prediction failed");
        println!("Prediction result: {:?}", prediction.unwrap());
    }

    #[test]
    fn test_model_classification() {
        let model_path = "models/example_model.onnx"; // Replace with a real ONNX model path
        let model = MLModel::new("classification_model", model_path).expect("Failed to load model");

        let input_data = vec![0.2, 0.5, 0.3];
        let class_labels = vec!["Class A", "Class B", "Class C"];

        let classification = model.classify(&input_data, &class_labels);

        assert!(classification.is_ok(), "Classification failed");
        println!("Classification result: {:?}", classification.unwrap());
    }
}
