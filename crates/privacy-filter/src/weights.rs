use crate::{Error, Result};
use burn::{
    module::Param,
    nn::{Linear, RmsNorm},
    tensor::{Bytes, DType, Tensor, TensorData, backend::Backend},
};
use burn_store::{ModuleStore, SafetensorsStore, TensorSnapshot};
use std::{collections::BTreeMap, path::Path};

/// 权重只在模型构建期间读取、转换并上传；推理热路径不再访问文件。
pub(crate) struct Weights<B: Backend> {
    snapshots: BTreeMap<String, TensorSnapshot>,
    device: B::Device,
}

impl<B: Backend> Weights<B> {
    pub fn open(path: impl AsRef<Path>, device: &B::Device) -> Result<Self> {
        let mut store = SafetensorsStore::from_file(path.as_ref());
        Ok(Self {
            snapshots: store
                .get_all_snapshots()
                .map_err(|e| Error::InvalidModel(e.to_string()))?
                .clone(),
            device: device.clone(),
        })
    }

    fn data<const D: usize>(&self, name: &str, shape: [usize; D]) -> Result<TensorData> {
        let snapshot = self
            .snapshots
            .get(name)
            .ok_or_else(|| Error::InvalidModel(format!("missing tensor {name}")))?;
        if snapshot.shape.as_slice() != shape
            || !matches!(snapshot.dtype, DType::F32 | DType::BF16 | DType::F16)
        {
            return Err(Error::InvalidModel(format!(
                "invalid shape/dtype for {name}: {:?}, {:?}",
                snapshot.shape, snapshot.dtype
            )));
        }
        snapshot
            .to_data()
            .map_err(|e| Error::InvalidModel(e.to_string()))
    }

    pub fn tensor<const D: usize>(&self, name: &str, shape: [usize; D]) -> Result<Tensor<B, D>> {
        Ok(Tensor::from_data(
            self.data(name, shape)?.convert::<f32>(),
            &self.device,
        ))
    }

    pub fn linear(&self, prefix: &str, input: usize, output: usize) -> Result<Linear<B>> {
        Ok(Linear {
            weight: Param::from_tensor(
                self.tensor(&format!("{prefix}.weight"), [output, input])?
                    .transpose(),
            ),
            bias: Some(Param::from_tensor(
                self.tensor(&format!("{prefix}.bias"), [output])?,
            )),
        })
    }

    pub fn norm(&self, prefix: &str, hidden: usize, epsilon: f32) -> Result<RmsNorm<B>> {
        Ok(RmsNorm {
            gamma: Param::from_tensor(self.tensor(&format!("{prefix}.weight"), [hidden])?),
            epsilon: epsilon as f64,
        })
    }

    /// 沿专家轴拆分后上传，避免单个巨型 GPU storage buffer 的尺寸限制。
    pub fn experts<const D: usize, const R: usize>(
        &self,
        name: &str,
        shape: [usize; R],
    ) -> Result<Vec<Tensor<B, D>>> {
        let data = self.data(name, shape)?;
        let row_shape: [usize; D] = data.shape.as_slice()[1..]
            .try_into()
            .map_err(|_| Error::InvalidModel("invalid expert rank".into()))?;
        let bytes_per_row = row_shape.iter().product::<usize>() * data.dtype.size();
        Ok(data
            .bytes
            .chunks_exact(bytes_per_row)
            .map(|bytes| {
                Tensor::from_data(
                    TensorData {
                        bytes: Bytes::from_bytes_vec(bytes.to_vec()),
                        shape: row_shape.into(),
                        dtype: data.dtype,
                    }
                    .convert::<f32>(),
                    &self.device,
                )
            })
            .collect())
    }
}
