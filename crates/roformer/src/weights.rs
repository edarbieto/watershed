//! Reads weights from a PyTorch checkpoint, without running its pickle code.

use std::path::Path;

use burn::prelude::*;
use burn::tensor::{DType, TensorData};
use pytorch_reader::PytorchReader;

#[derive(Debug, thiserror::Error)]
pub enum WeightsError {
    #[error("cannot read the checkpoint: {0}")]
    Read(#[from] pytorch_reader::PytorchError),
    #[error("cannot read a tensor: {0}")]
    Io(#[from] std::io::Error),
    #[error("the checkpoint has no tensor {0}")]
    Missing(String),
    #[error("{name} is {got:?}, expected f32 with {expected} dimensions")]
    Shape {
        name: String,
        got: Vec<usize>,
        expected: usize,
    },
}

/// A checkpoint open for reading; tensors are read from the file on demand.
pub struct Weights {
    reader: PytorchReader,
}

impl Weights {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, WeightsError> {
        Ok(Self {
            reader: PytorchReader::new(path)?,
        })
    }

    pub fn has(&self, name: &str) -> bool {
        self.reader.get(name).is_some()
    }

    /// The tensor's values and shape.
    pub fn read(&self, name: &str) -> Result<(Vec<f32>, Vec<usize>), WeightsError> {
        let tensor = self
            .reader
            .get(name)
            .ok_or_else(|| WeightsError::Missing(name.into()))?;
        if tensor.dtype() != pytorch_reader::DType::F32 {
            return Err(WeightsError::Shape {
                name: name.into(),
                got: tensor.shape().to_vec(),
                expected: tensor.shape().len(),
            });
        }
        let values = tensor
            .read()?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        Ok((values, tensor.shape().to_vec()))
    }
}

/// Where a part of the model loads from, and on what device and precision it runs.
#[derive(Clone, Copy)]
pub struct Source<'a> {
    pub weights: &'a Weights,
    pub device: &'a Device,
    pub dtype: DType,
}

impl Source<'_> {
    pub fn tensor<const D: usize>(&self, name: &str) -> Result<Tensor<D>, WeightsError> {
        let (values, shape) = self.weights.read(name)?;
        if shape.len() != D {
            return Err(WeightsError::Shape {
                name: name.into(),
                got: shape,
                expected: D,
            });
        }
        let tensor: Tensor<D> = Tensor::from_data(TensorData::new(values, shape), self.device);
        Ok(tensor.cast(self.dtype))
    }
}
