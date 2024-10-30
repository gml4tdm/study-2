//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Binary Metrics
//////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BinaryClassificationMetrics {
    pub confusion_matrix: BinaryConfusionMatrix,
}


#[allow(unused)]
impl BinaryClassificationMetrics {
    pub fn new(predictions: &[bool], ground_truths: &[bool]) -> Self {
        let confusion = BinaryConfusionMatrix::new(predictions, ground_truths);
        Self{confusion_matrix: confusion}
    }
    
    pub fn from_confusion_matrix(mat: BinaryConfusionMatrix) -> Self {
        Self { confusion_matrix: mat }
    }

    pub fn accuracy(&self) -> f64 { 
        (self.confusion_matrix.correct() as f64) / (self.confusion_matrix.total() as f64)
    }

    pub fn precision(&self) -> f64 {
        (self.confusion_matrix.true_positives as f64) / (self.confusion_matrix.predicted_positive() as f64)
    }

    pub fn recall(&self) -> f64 {
        (self.confusion_matrix.true_positives as f64) / (self.confusion_matrix.actually_positive() as f64)
    }

    pub fn true_positive_rate(&self) ->  f64 {
        self.recall()
    }

    pub fn sensitivity(&self) -> f64 {
        self.recall()
    }

    pub fn f1_score(&self) -> f64 {
        let precision = self.precision();
        let recall = self.recall();
        2.0 * precision * recall / (precision + recall)
    }

    pub fn specificity(&self) -> f64 {
        (self.confusion_matrix.true_negatives as f64) / (self.confusion_matrix.actually_negative() as f64)
    }

    pub fn true_negative_rate(&self) -> f64 {
        self.sensitivity()
    }

    pub fn false_positive_rate(&self) -> f64 {
        (self.confusion_matrix.false_positives as f64) / (self.confusion_matrix.actually_negative() as f64)
    }

    pub fn false_negative_rate(&self) -> f64 {
        (self.confusion_matrix.false_negatives as f64) / (self.confusion_matrix.actually_positive() as f64)
    }

    pub fn balanced_accuracy(&self) -> f64 {
        (self.true_positive_rate() + self.true_negative_rate()) / 2.0
    }

    pub fn prevalence(&self) -> f64 {
        (self.confusion_matrix.actually_positive() as f64) / (self.confusion_matrix.total() as f64)
    }

    pub fn matthews_correlation_coefficient(&self) -> f64 {
        let denominator_squared = self.confusion_matrix.predicted_positive() *
            self.confusion_matrix.actually_positive() * 
            self.confusion_matrix.actually_negative() * 
            self.confusion_matrix.predicted_negative();
        let denominator = (denominator_squared as f64).sqrt();
        let numerator_lhs = self.confusion_matrix.true_positives * self.confusion_matrix.true_negatives;
        let numerator_rhs = self.confusion_matrix.false_positives * self.confusion_matrix.false_negatives;
        let numerator = (numerator_lhs + numerator_rhs) as f64;
        numerator / denominator
    }

    pub fn cohen_kappa(&self) -> f64 {
        let numerator_lhs = self.confusion_matrix.true_positives * self.confusion_matrix.true_negatives;
        let numerator_rhs = self.confusion_matrix.false_positives * self.confusion_matrix.false_negatives;
        let numerator = 2.0 * (numerator_lhs + numerator_rhs) as f64;
        let denominator_lhs = self.confusion_matrix.predicted_positive() * self.confusion_matrix.actually_negative();
        let denominator_rhs = self.confusion_matrix.predicted_negative() * self.confusion_matrix.actually_positive();
        let denominator = (denominator_lhs + denominator_rhs) as f64;
        numerator / denominator
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Binary Confusion Matrix
//////////////////////////////////////////////////////////////////////////////////////////////////


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BinaryConfusionMatrix {
    pub true_positives: u64,
    pub false_positives: u64,
    pub false_negatives: u64,
    pub true_negatives: u64,
}


impl BinaryConfusionMatrix { 
    pub fn new(predictions: &[bool], ground_truths: &[bool]) -> Self {
        if predictions.len() != ground_truths.len() {
            panic!("Predictions and ground truths must have equal length!")
        }
        let mut true_positives = 0;
        let mut false_positives = 0;
        let mut false_negatives = 0;
        let mut true_negatives = 0;
        let stream = predictions.iter().copied()
            .zip(ground_truths.iter().copied());
        for (prediction, truth) in stream {
            match (prediction, truth) {
                (true, true) => { true_positives += 1; }
                (true, false) => { false_positives += 1; }
                (false, true) => { false_negatives += 1; }
                (false, false) => { true_negatives += 1; }
            }
        }
        Self { true_positives, false_positives, false_negatives, true_negatives }
    }

    #[allow(unused)]
    pub fn from_counts(true_positives: u64,
                       false_positives: u64, 
                       true_negatives: u64, 
                       false_negatives: u64) -> Self
    {
        Self { true_positives, false_negatives, true_negatives, false_positives }
    }
    
    pub fn total(&self) -> u64 {
        self.true_positives + 
            self.false_positives + 
            self.false_negatives +
            self.true_negatives
    }
    
    pub fn correct(&self) -> u64 {
        self.true_positives + self.true_negatives
    }
    
    #[allow(unused)]
    pub fn incorrect(&self) -> u64 {
        self.false_negatives + self.false_positives
    }
    
    pub fn predicted_positive(&self) -> u64 {
        self.true_positives + self.false_positives
    }
    
    pub fn predicted_negative(&self) -> u64 {
        self.false_negatives + self.true_negatives
    }
    
    pub fn actually_positive(&self) -> u64 {
        self.true_positives + self.false_negatives
    }
    
    pub fn actually_negative(&self) -> u64 {
        self.true_negatives + self.false_positives
    }
}
