use crate::{
    BatchLimits, Entity, Error, Options, PrivacyFilter, Result, TokenPrediction,
    observation::{InputWork, Observation, Run, Stage},
};
use burn::tensor::backend::Backend;

/// 两种公开输出共用一个执行生命周期，防止观测、预算或微批次逻辑分叉。
pub(crate) trait InferenceOutput: Sized {
    fn from_predictions(
        text: &str,
        tokens: Vec<TokenPrediction>,
        options: &Options,
    ) -> Result<Vec<Self>>;
}

impl InferenceOutput for TokenPrediction {
    fn from_predictions(_: &str, tokens: Vec<TokenPrediction>, _: &Options) -> Result<Vec<Self>> {
        Ok(tokens)
    }
}

impl InferenceOutput for Entity {
    fn from_predictions(
        text: &str,
        tokens: Vec<TokenPrediction>,
        options: &Options,
    ) -> Result<Vec<Self>> {
        options.decoding.decode(text, &tokens)
    }
}

impl<B: Backend> PrivacyFilter<B> {
    pub(crate) fn infer<T: InferenceOutput>(
        &self,
        texts: &[&str],
        limits: BatchLimits,
        observation: Option<&Observation<'_>>,
    ) -> Result<Vec<Vec<T>>> {
        let mut run = observation.map(|observation| Run::start(observation, texts.len()));
        let result = self.execute(texts, limits, &mut run);
        if let Some(run) = run {
            run.finish(result.is_ok());
        }
        result
    }

    fn execute<T: InferenceOutput>(
        &self,
        texts: &[&str],
        limits: BatchLimits,
        run: &mut Option<Run<'_>>,
    ) -> Result<Vec<Vec<T>>> {
        if let Some(run) = run {
            run.begin_stage(Stage::Validation);
        }
        limits.validate()?;
        if let Some(run) = run {
            run.end_stage();
        }
        if let Some(run) = run {
            run.begin_stage(Stage::Tokenization);
        }
        let encodings = self
            .tokenizer
            .encode_batch(texts.to_vec(), false)
            .map_err(|e| Error::Tokenizer(e.to_string()))?;
        if let Some(run) = run {
            run.end_stage();
            run.tokenized(encodings.iter().map(|encoding| encoding.len()));
        }
        if let Some(run) = run {
            run.begin_stage(Stage::Validation);
        }
        let limit = self.options.max_tokens.min(limits.max_tokens);
        for encoding in &encodings {
            if encoding.len() > limit {
                return Err(Error::InputTooLong {
                    actual: encoding.len(),
                    limit,
                });
            }
        }
        let mut output: Vec<Vec<T>> = (0..texts.len()).map(|_| Vec::new()).collect();
        // 全部输入先校验，然后执行推理；空输入不占用设备批次。
        let plan = MicroBatches::new(&encodings, limits);
        if let Some(run) = run {
            run.end_stage();
            run.planned(plan.ranges.len());
        }
        if let Some(run) = run {
            for (index, encoding) in encodings
                .iter()
                .enumerate()
                .filter(|(_, encoding)| encoding.is_empty())
            {
                run.input_completed(index, encoding.len());
            }
        }
        for indices in plan.iter() {
            let sequences: Vec<&[u32]> = indices.iter().map(|&i| encodings[i].get_ids()).collect();
            if let Some(run) = run {
                run.batch_started(
                    indices
                        .iter()
                        .map(|&index| InputWork {
                            index,
                            tokens: encodings[index].len(),
                        })
                        .collect(),
                );
                run.begin_stage(Stage::Prefill);
            }
            // forward 内部的结果回传是已有同步点；观测不会额外等待设备。
            let mut logits = self
                .model
                .forward(&sequences, |submitted, total| {
                    if let Some(run) = run {
                        run.layer_submitted(submitted, total);
                    }
                })?
                .into_iter();
            if let Some(run) = run {
                run.end_stage();
            }
            for &index in indices {
                if let Some(run) = run {
                    run.begin_stage(Stage::Decoding);
                }
                let predictions = encodings[index]
                    .get_offsets()
                    .iter()
                    .map(|&(start, end)| TokenPrediction {
                        offsets: start..end,
                        logits: logits.next().expect("模型输出与输入 token 数一致"),
                    })
                    .collect();
                output[index] = T::from_predictions(texts[index], predictions, &self.options)?;
                if let Some(run) = run {
                    run.end_stage();
                    run.input_completed(index, encodings[index].len());
                }
            }
            if let Some(run) = run {
                run.batch_completed();
            }
        }
        Ok(output)
    }
}

/// 预算规划与实际执行使用同一份边界，观测不会另算一份批次数。
struct MicroBatches {
    indices: Vec<usize>,
    ranges: Vec<std::ops::Range<usize>>,
}

impl MicroBatches {
    fn new(encodings: &[tokenizers::Encoding], limits: BatchLimits) -> Self {
        let indices: Vec<_> = encodings
            .iter()
            .enumerate()
            .filter(|(_, encoding)| !encoding.is_empty())
            .map(|(index, _)| index)
            .collect();
        let mut ranges = Vec::new();
        let mut cursor = 0;
        while cursor < indices.len() {
            let first = cursor;
            let mut tokens = 0;
            while cursor < indices.len() && cursor - first < limits.max_sequences {
                let length = encodings[indices[cursor]].len();
                if tokens + length > limits.max_tokens {
                    break;
                }
                tokens += length;
                cursor += 1;
            }
            ranges.push(first..cursor);
        }
        Self { indices, ranges }
    }

    fn iter(&self) -> impl Iterator<Item = &[usize]> {
        self.ranges.iter().map(|range| &self.indices[range.clone()])
    }
}
