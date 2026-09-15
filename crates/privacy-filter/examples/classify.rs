use privacy_filter::{Decoding, Options, PrivacyFilter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .unwrap_or_else(|| "models/privacy-filter".into());
    let text = args.next().unwrap_or_else(|| {
        "My name is Harry Potter and my email is harry.potter@hogwarts.edu.".into()
    });
    let options = Options {
        decoding: if args.any(|a| a == "--viterbi") {
            Decoding::Viterbi
        } else {
            Decoding::Simple
        },
        ..Options::default()
    };
    let classifier = PrivacyFilter::from_dir_with_options(path, options)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&classifier.classify(&text)?)?
    );
    Ok(())
}
