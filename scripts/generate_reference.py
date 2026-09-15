"""用固定版本 Transformers 生成独立参考值；仅开发验证需要 Python。"""

import argparse
import json
from pathlib import Path

import torch
from tokenizers import Tokenizer
from tokenizers.models import WordLevel
from tokenizers.pre_tokenizers import Whitespace
from transformers import AutoModelForTokenClassification, AutoTokenizer, OpenAIPrivacyFilterConfig, OpenAIPrivacyFilterForTokenClassification


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--real", action="store_true", help="生成真实模型的参考值")
    parser.add_argument("--model-dir", default="models/privacy-filter")
    args = parser.parse_args()
    torch.set_num_threads(1)
    torch.manual_seed(42)
    root = Path(__file__).resolve().parents[1]
    destination = root / "crates/privacy-filter/tests/fixtures/tiny"
    destination.mkdir(parents=True, exist_ok=True)
    if args.real:
        tokenizer = AutoTokenizer.from_pretrained(args.model_dir)
        model = AutoModelForTokenClassification.from_pretrained(
            args.model_dir, dtype=torch.float32, attn_implementation="eager").eval()
        text = "My name is Harry Potter and my email is harry.potter@hogwarts.edu."
        with torch.no_grad():
            logits = model(**tokenizer(text, return_tensors="pt")).logits[0].tolist()
        output = destination.parent / "privacy-reference.json"
        output.write_text(json.dumps({"text": text, "logits": logits}, separators=(",", ":")))
        return
    # 标签直接取自已提交的参考 checkpoint 配置，不再维护第二份标签表。
    source_config = json.loads((destination / "config.json").read_text())
    config = OpenAIPrivacyFilterConfig(
        vocab_size=32, hidden_size=16, intermediate_size=12, head_dim=8,
        num_attention_heads=4, num_key_value_heads=2, num_hidden_layers=2,
        num_local_experts=4, num_experts_per_tok=2, sliding_window=3,
        max_position_embeddings=131072, pad_token_id=31, eos_token_id=31,
        id2label=source_config["id2label"], label2id=source_config["label2id"],
        rope_parameters={"rope_type": "yarn", "rope_theta": 150000.0,
                         "factor": 32.0, "beta_fast": 32.0, "beta_slow": 1.0,
                         "original_max_position_embeddings": 4096, "truncate": False},
    )
    config._attn_implementation = "eager"
    model = OpenAIPrivacyFilterForTokenClassification(config).eval()
    # 加大随机权重以充分覆盖专家门控和不同路由，避免只验证接近零的输出。
    with torch.no_grad():
        for name, parameter in model.named_parameters():
            if "norm" not in name:
                parameter.normal_(mean=0.0, std=0.2)
    model.save_pretrained(destination)
    tokenizer = Tokenizer(WordLevel({f"t{i}": i for i in range(config.vocab_size)}, unk_token="t31"))
    tokenizer.pre_tokenizer = Whitespace()
    tokenizer.save(str(destination / "tokenizer.json"))
    cases = []
    # 133 个 token 跨越 Rust 的 128 查询块边界，验证窗口拼接没有丢失上下文。
    for length in [1, 7, 133]:
        ids = torch.randint(0, 31, (1, length))
        with torch.no_grad():
            logits = model(input_ids=ids).logits[0].tolist()
        cases.append({"text": " ".join(f"t{i}" for i in ids[0].tolist()), "logits": logits})
    (destination / "reference.json").write_text(json.dumps(cases, separators=(",", ":")))


if __name__ == "__main__":
    main()
