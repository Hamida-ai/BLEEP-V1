#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    struct MockAIModel;

    impl AIModel for MockAIModel {
        fn predict(&self, input: &[f32]) -> Result<PredictionResult, BLEEPError> {
            if input.is_empty() {
                return Err(BLEEPError::InvalidInput);
            }
            Ok(PredictionResult::FloatVec(input.to_vec()))
        }
    }

    #[test]
    fn test_model_registration() {
        let rt = Runtime::new().unwrap();
        let mut ai_module = BLEEPAIDecisionModule::new();
        let model = Arc::new(MockAIModel);

        let result = rt.block_on(ai_module.register_model("test_model".to_string(), model.clone()));
        assert!(result.is_ok(), "Model registration should succeed");

        let duplicate_result =
            rt.block_on(ai_module.register_model("test_model".to_string(), model.clone()));
        assert_eq!(
            duplicate_result,
            Err(BLEEPError::ModelAlreadyRegistered),
            "Duplicate model registration should fail"
        );
    }

    #[test]
    fn test_model_prediction() {
        let rt = Runtime::new().unwrap();
        let mut ai_module = BLEEPAIDecisionModule::new();
        let model = Arc::new(MockAIModel);

        rt.block_on(ai_module.register_model("test_model".to_string(), model))
            .unwrap();

        let input_data = vec![0.1, 0.5, 0.9];
        let result = rt.block_on(ai_module.predict("test_model", &input_data));

        assert!(
            matches!(result, Ok(PredictionResult::FloatVec(_))),
            "Prediction should return a FloatVec result"
        );
    }

    #[test]
    fn test_invalid_model_prediction() {
        let rt = Runtime::new().unwrap();
        let ai_module = BLEEPAIDecisionModule::new();
        let result = rt.block_on(ai_module.predict("unknown_model", &[1.0, 2.0]));

        assert_eq!(
            result,
            Err(BLEEPError::ModelNotFoundError),
            "Predicting with an unknown model should return an error"
        );
    }

    #[test]
    fn test_prediction_cache() {
        let rt = Runtime::new().unwrap();
        let mut ai_module = BLEEPAIDecisionModule::new();
        let model = Arc::new(MockAIModel);

        rt.block_on(ai_module.register_model("test_model".to_string(), model))
            .unwrap();

        let input_data = vec![0.3, 0.6, 0.9];
        let first_result = rt.block_on(ai_module.predict("test_model", &input_data));
        let second_result = rt.block_on(ai_module.predict("test_model", &input_data));

        assert_eq!(
            first_result, second_result,
            "Second prediction should use cached result"
        );
    }

    #[test]
    fn test_ensemble_majority_vote() {
        let model1 = Arc::new(MockAIModel);
        let model2 = Arc::new(MockAIModel);
        let ensemble = EnsemblePredictiveModel::new(
            vec![model1, model2],
            AggregationStrategy::MajorityVote,
        );

        let input_data = vec![1.0, 2.0, 3.0];
        let result = ensemble.predict(&input_data);

        assert!(
            matches!(result, Ok(PredictionResult::FloatVec(_))),
            "Majority vote should return a FloatVec result"
        );
    }

    #[test]
    fn test_ensemble_average() {
        let model1 = Arc::new(MockAIModel);
        let model2 = Arc::new(MockAIModel);
        let ensemble = EnsemblePredictiveModel::new(vec![model1, model2], AggregationStrategy::Average);

        let input_data = vec![1.0, 2.0, 3.0];
        let result = ensemble.predict(&input_data);

        assert!(
            matches!(result, Ok(PredictionResult::FloatVec(_))),
            "Averaging should return a FloatVec result"
        );
    }

    #[test]
    fn test_ensemble_weighted_average() {
        let model1 = Arc::new(MockAIModel);
        let model2 = Arc::new(MockAIModel);
        let weights = vec![0.7, 0.3];
        let ensemble = EnsemblePredictiveModel::new(
            vec![model1, model2],
            AggregationStrategy::WeightedAverage(weights),
        );

        let input_data = vec![1.0, 2.0, 3.0];
        let result = ensemble.predict(&input_data);

        assert!(
            matches!(result, Ok(PredictionResult::FloatVec(_))),
            "Weighted averaging should return a FloatVec result"
        );
    }

    #[test]
    fn test_timeout_handling() {
        let rt = Runtime::new().unwrap();
        let mut ai_module = BLEEPAIDecisionModule::new();
        let model = Arc::new(MockAIModel);

        rt.block_on(ai_module.register_model("test_model".to_string(), model))
            .unwrap();

        let input_data = vec![1.0, 2.0, 3.0];
        let result = rt.block_on(ai_module.predict("test_model", &input_data));

        assert!(
            result.is_ok(),
            "Prediction should not timeout under normal conditions"
        );
    }
      } 
