//! 请求内容的处理管线：定位 → 识别 → 判定 → 替换，并产出待入库的审计内容。
//!
//! 本模块是同步的：识别是阻塞调用，由 HTTP 层放到阻塞线程池执行。这样管线本身可以在没有
//! 运行时的测试里直接调用，也保证「先完成替换、再转发」的顺序无法被异步调度打乱。

use crate::Error;
use crate::classifier::Classifier;
use crate::protocol::{adapter, apply};
use crate::redaction::{DecidedSpan, redact};
use privacy_filter::Filter;
use privacy_rules::{Action, RuleSet};
use privacy_store::{ApiFormat, FragmentRecording, RecordedSpan};
use serde_json::Value;

/// 一次请求的定位信息，进入审计记录与日志。
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub api_format: ApiFormat,
    pub path: String,
}

/// 处理结果：可直接转发的报文，以及待入库的审计内容。
#[derive(Debug)]
pub struct Processed {
    /// 脱敏后的报文。没有任何内容字段时与入参逐字节相同。
    pub body: Value,
    /// 只包含**本次新增**且真的产生了判定的片段：没有命中的文本不入池，此前已经审过的
    /// turn（命中持久化链）也不重复入池。
    pub fragments: Vec<FragmentRecording>,
    pub usage: Workload,
    pub released: usize,
    pub redacted: usize,
}

/// 本次识别的工作量，用于日志与性能统计。
#[derive(Debug, Clone, Copy, Default)]
pub struct Workload {
    pub performance: privacy_filter::performance::ModelPerformance,
    pub cached_fragments: i64,
    pub deduplicated_fragments: i64,
    pub inferred_fragments: i64,
    pub inference_ms: i64,
}

impl Processed {
    fn untouched(body: Value) -> Self {
        Self {
            body,
            fragments: Vec::new(),
            usage: Workload::default(),
            released: 0,
            redacted: 0,
        }
    }
}

/// 处理一个已经解析成 JSON 的请求体。
///
/// 任何一步失败都返回错误，由调用方拒绝请求——**不返回未脱敏的报文**，因此不存在「失败就
/// 原样转发」的降级路径。
pub fn process(
    body: &Value,
    context: &RequestContext,
    classifier: &dyn Classifier,
    rules: &RuleSet,
) -> Result<Processed, Error> {
    let fields = adapter(context.api_format).collect(body)?;
    if fields.is_empty() {
        return Ok(Processed::untouched(body.clone()));
    }

    let texts: Vec<&str> = fields.iter().map(|field| field.text.as_str()).collect();
    let classification = classifier.classify_recorded(&texts)?;
    if classification.fields.len() != fields.len() {
        // 识别结果与字段数量对不上意味着结果可能错位；错位替换比重发更危险。
        return Err(Error::ClassificationShape {
            fields: fields.len(),
            results: classification.fields.len(),
        });
    }

    let mut replacements: Vec<(String, String)> = Vec::new();
    let mut fragments = Vec::new();
    let mut released = 0;
    let mut redacted = 0;
    // 管理员的登记与模型互不知晓：登记的原文出现在文本里就是命中，出现几次算几次，
    // 与模型有没有提议、把那段切得多长都无关。判定与替换照旧共用同一套规则。
    let registrations = rules.registrations();

    for (field, classified) in fields.iter().zip(&classification.fields) {
        let mut entities = classified.entities.clone();
        for registration in &registrations {
            entities.extend(registration.recognize(&field.text));
        }
        let outcome = redact(&field.text, &entities, rules);
        if outcome.spans.is_empty() {
            continue;
        }

        released += outcome
            .spans
            .iter()
            .filter(|span| span.action == Action::Release)
            .count();
        redacted += outcome
            .spans
            .iter()
            .filter(|span| span.action == Action::Redact)
            .count();

        // 只有本次新增的 turn 入池。请求每次都会把整段历史重发一遍，命中持久化链的内容
        // 在更早的请求里已经判过，逐次入池只会把池子淹掉；判定与替换本身照常进行。
        // 放行与抹去都入库：控制台要能看到全部判定，而不只是被改动的部分。
        if !classified.already_audited {
            fragments.push(FragmentRecording {
                request_id: context.request_id.clone(),
                api_format: context.api_format,
                path: context.path.clone(),
                content_hash: blake3::hash(field.text.as_bytes()).as_bytes().to_vec(),
                original_text: field.text.clone(),
                redacted_text: outcome.redacted.clone(),
                spans: outcome.spans.iter().map(record).collect(),
            });
        }

        if outcome.redacted != field.text {
            replacements.push(field.replacement(outcome.redacted));
        }
    }

    let mut body = body.clone();
    apply(&mut body, &replacements)?;

    Ok(Processed {
        body,
        fragments,
        usage: Workload {
            performance: classification.performance,
            cached_fragments: classification.usage.cached_fragments as i64,
            deduplicated_fragments: classification.usage.deduplicated_fragments as i64,
            inferred_fragments: classification.usage.inferred_fragments as i64,
            inference_ms: classification.elapsed_ms,
        },
        released,
        redacted,
    })
}

fn record(span: &DecidedSpan) -> RecordedSpan {
    RecordedSpan {
        id: String::new(),
        entity_group: span.entity_group,
        score: span.score,
        char_len: span.original_text.chars().count(),
        byte_start: span.byte_start,
        byte_end: span.byte_end,
        original_text: span.original_text.clone(),
        action: span.action,
        matched_rule_id: span.matched_rule_id.clone(),
    }
}
