//! 手动性能基准；可读取临时 JSON 片段文件，只输出数量和时间，不输出原文。
use burn::tensor::backend::Backend;
use privacy_filter::{BatchLimits, Options, PrivacyFilter};
use privacy_filter_cache::{CacheLimits, CachedPrivacyFilter};
use serde::Deserialize;
use std::time::Instant;

#[derive(Deserialize)]
struct Fragment {
    text: String,
}

struct Benchmark {
    texts: Vec<String>,
    full: bool,
    warm: bool,
    max_tokens: usize,
}

impl Benchmark {
    fn run<B: Backend>(&self, device: &B::Device) -> Result<(), Box<dyn std::error::Error>> {
        let load = Instant::now();
        let model = PrivacyFilter::<B>::from_dir_on_device(
            "models/privacy-filter",
            Options {
                max_tokens: self.max_tokens,
                ..Default::default()
            },
            device,
        )?;
        println!("load_ms={}", load.elapsed().as_millis());
        let texts: Vec<&str> = self.texts.iter().map(String::as_str).collect();
        let limits = BatchLimits {
            max_sequences: 16,
            max_tokens: self.max_tokens,
        };
        // 首次调用包含内核编译；单独报告，不混入热启动数据。
        let warmup = Instant::now();
        std::hint::black_box(model.classify(texts[0])?);
        println!("warmup_ms={}", warmup.elapsed().as_millis());
        if !self.full {
            let start = Instant::now();
            for text in &texts {
                std::hint::black_box(model.classify(text)?);
            }
            println!("sequential_ms={}", start.elapsed().as_millis());
            let start = Instant::now();
            std::hint::black_box(model.classify_batch(&texts, limits)?);
            println!("batch_ms={}", start.elapsed().as_millis());
        }
        let mut cached = CachedPrivacyFilter::new(
            model,
            CacheLimits {
                max_entries: 4096,
                max_bytes: 64 * 1024 * 1024,
            },
        )?;
        let prefix = texts.len().saturating_sub(1);
        for (name, input) in [
            ("prefix_cold", &texts[..prefix]),
            ("append_one", &texts[..]),
            ("replay", &texts[..]),
        ] {
            let start = Instant::now();
            let result = cached.classify_batch(input, limits)?;
            println!(
                "phase={name} fragments={} elapsed_ms={} usage={}",
                input.len(),
                start.elapsed().as_millis(),
                serde_json::to_string(&result.usage)?
            );
        }
        if self.warm {
            cached.clear_cache();
            let start = Instant::now();
            let result = cached.classify_batch(&texts, limits)?;
            println!(
                "phase=kernel_warm_uncached elapsed_ms={} usage={}",
                start.elapsed().as_millis(),
                serde_json::to_string(&result.usage)?
            );
            for round in 0..3 {
                cached.clear_cache();
                let start = Instant::now();
                let result = cached.classify_batch(&texts[texts.len() - 1..], limits)?;
                println!(
                    "phase=kernel_warm_single round={round} elapsed_us={} usage={}",
                    start.elapsed().as_micros(),
                    serde_json::to_string(&result.usage)?
                );
            }
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let file = args.iter().find(|a| !a.starts_with("--"));
    let texts = if let Some(path) = file {
        serde_json::from_reader::<_, Vec<Fragment>>(std::io::BufReader::new(std::fs::File::open(
            path,
        )?))?
        .into_iter()
        .map(|f| f.text)
        .filter(|s| !s.is_empty())
        .collect()
    } else {
        [
            "My name is Harry Potter and my email is harry.potter@hogwarts.edu.",
            "Contact Alice Smith at alice@example.com.",
            "The tool returned a successful result with no matching records.",
            "Please send the file to bob@example.org.",
        ]
        .map(String::from)
        .to_vec()
    };
    let benchmark = Benchmark {
        texts,
        full: args.iter().any(|a| a == "--full"),
        warm: args.iter().any(|a| a == "--warm"),
        max_tokens: 8192,
    };
    if benchmark.texts.is_empty() {
        return Err("片段列表不能为空".into());
    }
    #[cfg(feature = "gpu")]
    if args.iter().any(|a| a == "--gpu") {
        return benchmark.run::<privacy_filter::Gpu>(&Default::default());
    }
    benchmark.run::<privacy_filter::Cpu<f32>>(&Default::default())
}
