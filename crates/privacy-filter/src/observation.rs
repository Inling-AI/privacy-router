//! 可选、按调用隔离的观测协议；不包含输入原文、token ID、实体或设备数据。
use std::time::{Duration, Instant};
use strum::EnumCount;

/// 分类推理的计时阶段。Prefill 表示输入前向计算，不是生成模型的 KV 预填充。
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumCount)]
#[repr(usize)]
pub enum Stage {
    Validation,
    Tokenization,
    Prefill,
    Decoding,
}

/// 阶段累计耗时；通过 Stage 查询，阶段数量由同一个枚举推导。
#[derive(Debug, Clone, Default)]
pub struct StageTimings([Duration; Stage::COUNT]);

impl StageTimings {
    pub fn duration(&self, stage: Stage) -> Duration {
        self.0[stage as usize]
    }
}

/// 当前调用的进度。Token 总量在分词结束前未知；空输入也计入完成输入数。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progress {
    pub total_inputs: usize,
    pub tokenized_inputs: usize,
    pub total_tokens: Option<usize>,
    pub completed_inputs: usize,
    pub completed_tokens: usize,
    pub completed_batches: usize,
    pub total_batches: Option<usize>,
}

/// 以结果真正回传到主机后的时间计量，不声称测得独立 GPU kernel 执行时间。
#[derive(Debug, Clone, Default)]
pub struct InferenceMetrics {
    pub progress: Progress,
    pub elapsed: Duration,
    pub stages: StageTimings,
    /// 第一个非空输入的公开结果在内部准备就绪的时间；不是网络首字节时间。
    /// 全空输入或第一个结果前失败时为 None。
    pub time_to_first_result: Option<Duration>,
}

impl InferenceMetrics {
    /// 已完成输入 token 数 / 累计前向耗时；零耗时或尚无完成 token 时不报告。
    pub fn input_tokens_per_second(&self) -> Option<f64> {
        let seconds = self.stages.duration(Stage::Prefill).as_secs_f64();
        (seconds > 0.0 && self.progress.completed_tokens > 0)
            .then(|| self.progress.completed_tokens as f64 / seconds)
    }

    /// 本模型不生成 token，因此生成 tg/s 不适用，不能用分类吞吐冒充。
    pub fn generation_tokens_per_second(&self) -> Option<f64> {
        None
    }

    /// 本模型不生成首个 token；需要分类响应延迟时读取 time_to_first_result。
    pub fn time_to_first_generated_token(&self) -> Option<Duration> {
        None
    }
}

/// 输入位置始终对应本次 API 调用的原始下标，不受微批次划分影响。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputWork {
    pub index: usize,
    pub tokens: usize,
}

/// 事件种类可扩展；消费者应保留通配分支。
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum EventKind {
    Started,
    InputTokenized(InputWork),
    /// 输入预算校验通过，微批次总数已经确定。
    Planned,
    BatchStarted {
        index: usize,
        inputs: Vec<InputWork>,
    },
    /// GPU 可能尚未完成这些层的计算；观测不会为此插入设备同步。
    LayerSubmitted {
        batch: usize,
        submitted: usize,
        total: usize,
    },
    InputCompleted(InputWork),
    BatchCompleted {
        index: usize,
    },
    Completed,
    /// 返回的 Result 保留具体错误；事件只提供阶段，避免错误文本泄漏输入。
    Failed {
        stage: Stage,
    },
}

/// 一个时间点的完整快照。计数、耗时及吞吐均由同一个 metrics 状态导出。
#[derive(Debug, Clone)]
pub struct InferenceEvent {
    pub kind: EventKind,
    pub metrics: InferenceMetrics,
}

/// 同步回调，不绑定 tracing、OpenTelemetry、Prometheus 或 UI。
/// 回调应快速返回且不得 panic；耗时会进入端到端延迟。
/// 并发调用请分别创建 Observation，可在消费者中附加调用标识。
pub trait Observer {
    fn on_event(&self, event: &InferenceEvent);
}

/// 可注入的单调时钟；测试可用确定性时间代替宿主机时钟。
/// 返回值必须单调不减，起点可以任意。
pub trait Clock {
    fn now(&self) -> Duration;
}

/// 按一次调用借用观测器；默认使用单调时钟，也可注入测试时钟。
pub struct Observation<'a> {
    observer: &'a dyn Observer,
    clock: Option<&'a dyn Clock>,
}

impl<'a> Observation<'a> {
    pub fn new(observer: &'a dyn Observer) -> Self {
        Self {
            observer,
            clock: None,
        }
    }

    pub fn with_clock(observer: &'a dyn Observer, clock: &'a dyn Clock) -> Self {
        Self {
            observer,
            clock: Some(clock),
        }
    }
}

pub(crate) struct Run<'a> {
    observation: &'a Observation<'a>,
    timer: Timer<'a>,
    metrics: InferenceMetrics,
    stage: Stage,
    stage_started: Option<Duration>,
}

impl<'a> Run<'a> {
    pub fn start(observation: &'a Observation<'a>, inputs: usize) -> Self {
        let mut run = Self {
            observation,
            timer: Timer::new(observation.clock),
            metrics: InferenceMetrics {
                progress: Progress {
                    total_inputs: inputs,
                    ..Default::default()
                },
                ..Default::default()
            },
            stage: Stage::Validation,
            stage_started: None,
        };
        run.emit(EventKind::Started);
        run
    }

    fn elapsed(&self) -> Duration {
        self.timer.elapsed()
    }

    fn emit(&mut self, kind: EventKind) {
        self.metrics.elapsed = self.elapsed();
        // 快照包含进行中阶段的耗时，但不回写累计量，避免结束阶段时重复计时。
        let mut metrics = self.metrics.clone();
        if let Some(started) = self.stage_started {
            metrics.stages.0[self.stage as usize] += metrics.elapsed.saturating_sub(started);
        }
        self.observation
            .observer
            .on_event(&InferenceEvent { kind, metrics });
    }

    pub fn begin_stage(&mut self, stage: Stage) {
        self.stage = stage;
        self.stage_started = Some(self.elapsed());
    }

    pub fn end_stage(&mut self) {
        if let Some(started) = self.stage_started.take() {
            self.metrics.stages.0[self.stage as usize] += self.elapsed().saturating_sub(started);
        }
    }

    pub fn tokenized(&mut self, lengths: impl Iterator<Item = usize>) {
        let lengths: Vec<_> = lengths.collect();
        self.metrics.progress.total_tokens = Some(lengths.iter().sum());
        for (index, tokens) in lengths.into_iter().enumerate() {
            self.metrics.progress.tokenized_inputs += 1;
            self.emit(EventKind::InputTokenized(InputWork { index, tokens }));
        }
    }

    pub fn planned(&mut self, batches: usize) {
        self.metrics.progress.total_batches = Some(batches);
        self.emit(EventKind::Planned);
    }

    pub fn batch_started(&mut self, inputs: Vec<InputWork>) {
        self.emit(EventKind::BatchStarted {
            index: self.metrics.progress.completed_batches,
            inputs,
        });
    }

    pub fn layer_submitted(&mut self, submitted: usize, total: usize) {
        self.emit(EventKind::LayerSubmitted {
            batch: self.metrics.progress.completed_batches,
            submitted,
            total,
        });
    }

    pub fn input_completed(&mut self, index: usize, tokens: usize) {
        self.metrics.progress.completed_inputs += 1;
        self.metrics.progress.completed_tokens += tokens;
        if tokens > 0 && self.metrics.time_to_first_result.is_none() {
            self.metrics.time_to_first_result = Some(self.elapsed());
        }
        self.emit(EventKind::InputCompleted(InputWork { index, tokens }));
    }

    pub fn batch_completed(&mut self) {
        let index = self.metrics.progress.completed_batches;
        self.metrics.progress.completed_batches += 1;
        self.emit(EventKind::BatchCompleted { index });
    }

    pub fn finish(mut self, success: bool) {
        self.end_stage();
        self.emit(if success {
            EventKind::Completed
        } else {
            EventKind::Failed { stage: self.stage }
        });
    }
}

/// 时钟来源由枚举约束，不允许默认时钟与注入时钟处于矛盾状态。
enum Timer<'a> {
    System(Instant),
    Injected {
        clock: &'a dyn Clock,
        origin: Duration,
    },
}

impl<'a> Timer<'a> {
    fn new(clock: Option<&'a dyn Clock>) -> Self {
        match clock {
            Some(clock) => Self::Injected {
                clock,
                origin: clock.now(),
            },
            None => Self::System(Instant::now()),
        }
    }

    fn elapsed(&self) -> Duration {
        match self {
            Self::System(epoch) => epoch.elapsed(),
            Self::Injected { clock, origin } => clock.now().saturating_sub(*origin),
        }
    }
}
