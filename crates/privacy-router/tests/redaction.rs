mod common;

use common::{entity, redacting_rules};
use privacy_filter::Entity;
use privacy_router::redaction::redact;
use privacy_rules::{Action, EntityGroup, RuleSet};
use rstest::rstest;

#[rstest]
fn the_documented_example_is_redacted_exactly_as_specified(
    entity: impl Fn(EntityGroup, f64, &str, usize, &str) -> Entity,
    redacting_rules: RuleSet,
) {
    let text = "im AlexExample and my email is alex@example.com";
    let person = text.find("AlexExample").expect("存在");
    let email = text.find("alex@example.com").expect("存在");

    let result = redact(
        text,
        &[
            entity(
                EntityGroup::PrivatePerson,
                0.99,
                text,
                person,
                "AlexExample",
            ),
            entity(
                EntityGroup::PrivateEmail,
                0.99,
                text,
                email,
                "alex@example.com",
            ),
        ],
        &redacting_rules,
    );

    assert_eq!(
        result.redacted,
        "im <redacted_private_person> and my email is <redacted_private_email>"
    );
    assert_eq!(result.spans.len(), 2);
}

#[rstest]
fn surrounding_text_is_never_lost(
    entity: impl Fn(EntityGroup, f64, &str, usize, &str) -> Entity,
    redacting_rules: RuleSet,
) {
    let text = "prefix alice@corp.com suffix";
    let start = text.find("alice@corp.com").expect("存在");

    let result = redact(
        text,
        &[entity(
            EntityGroup::PrivateEmail,
            0.99,
            text,
            start,
            "alice@corp.com",
        )],
        &redacting_rules,
    );

    assert_eq!(result.redacted, "prefix <redacted_private_email> suffix");
}

#[rstest]
fn the_leading_space_of_a_matched_span_is_preserved(
    entity: impl Fn(EntityGroup, f64, &str, usize, &str) -> Entity,
    redacting_rules: RuleSet,
) {
    // 模型返回的实体常带前导空格；整段替换会让相邻词黏在一起。
    let text = "my name is Harry Potter!";
    let start = text.find(" Harry Potter").expect("存在");

    let result = redact(
        text,
        &[entity(
            EntityGroup::PrivatePerson,
            0.99,
            text,
            start,
            " Harry Potter",
        )],
        &redacting_rules,
    );

    assert_eq!(result.redacted, "my name is <redacted_private_person>!");
}

#[rstest]
fn repeated_values_are_replaced_at_their_own_positions(
    entity: impl Fn(EntityGroup, f64, &str, usize, &str) -> Entity,
    redacting_rules: RuleSet,
) {
    let text = "alice@corp.com wrote to alice@corp.com";
    let first = text.find("alice@corp.com").expect("存在");
    let second = text.rfind("alice@corp.com").expect("存在");
    assert_ne!(first, second);

    let result = redact(
        text,
        &[
            entity(
                EntityGroup::PrivateEmail,
                0.9,
                text,
                first,
                "alice@corp.com",
            ),
            entity(
                EntityGroup::PrivateEmail,
                0.8,
                text,
                second,
                "alice@corp.com",
            ),
        ],
        &redacting_rules,
    );

    assert_eq!(
        result.redacted,
        "<redacted_private_email> wrote to <redacted_private_email>"
    );
}

#[rstest]
fn multibyte_text_is_replaced_on_character_boundaries(
    entity: impl Fn(EntityGroup, f64, &str, usize, &str) -> Entity,
    redacting_rules: RuleSet,
) {
    let text = "联系人 李雷，邮箱 li@corp.com";
    let name = text.find("李雷").expect("存在");
    let email = text.find("li@corp.com").expect("存在");

    let result = redact(
        text,
        &[
            entity(EntityGroup::PrivatePerson, 0.99, text, name, "李雷"),
            entity(EntityGroup::PrivateEmail, 0.99, text, email, "li@corp.com"),
        ],
        &redacting_rules,
    );

    assert_eq!(
        result.redacted,
        "联系人 <redacted_private_person>，邮箱 <redacted_private_email>"
    );
}

#[rstest]
fn released_entities_keep_their_text_but_are_still_recorded(
    entity: impl Fn(EntityGroup, f64, &str, usize, &str) -> Entity,
) {
    let text = "contact user@example.com or alice@corp.com";
    let placeholder = text.find("user@example.com").expect("存在");
    let real = text.find("alice@corp.com").expect("存在");
    // 只放行指定地址；其余内容由默认保护规则抹去。
    let rules = RuleSet::new(
        RuleSet::builtin()
            .rules()
            .iter()
            .cloned()
            .chain([common::release_keyword("user@example.com")]),
    );

    let result = redact(
        text,
        &[
            entity(
                EntityGroup::PrivateEmail,
                0.99,
                text,
                placeholder,
                "user@example.com",
            ),
            entity(
                EntityGroup::PrivateEmail,
                0.99,
                text,
                real,
                "alice@corp.com",
            ),
        ],
        &rules,
    );

    assert_eq!(
        result.redacted,
        "contact user@example.com or <redacted_private_email>"
    );
    // 放行也是一次判定，必须留痕。
    assert_eq!(result.spans.len(), 2);
    assert!(
        result
            .spans
            .iter()
            .any(|span| span.action == Action::Release && span.original_text == "user@example.com")
    );
    assert!(result.changed());
}

#[rstest]
fn recorded_offsets_index_the_original_text(
    entity: impl Fn(EntityGroup, f64, &str, usize, &str) -> Entity,
    redacting_rules: RuleSet,
) {
    let text = "prefix alice@corp.com suffix";
    let start = text.find("alice@corp.com").expect("存在");
    let end = start + "alice@corp.com".len();

    let result = redact(
        text,
        &[entity(
            EntityGroup::PrivateEmail,
            0.99,
            text,
            start,
            "alice@corp.com",
        )],
        &redacting_rules,
    );

    let span = &result.spans[0];
    assert_eq!((span.byte_start, span.byte_end), (start, end));
    assert_eq!(&text[span.byte_start..span.byte_end], span.original_text);
}

#[rstest]
fn text_without_entities_is_returned_unchanged(redacting_rules: RuleSet) {
    let text = "nothing sensitive here";
    let result = redact(text, &[], &redacting_rules);

    assert_eq!(result.redacted, text);
    assert!(result.spans.is_empty());
    assert!(!result.changed());
}

#[rstest]
fn spans_are_reported_in_document_order(
    entity: impl Fn(EntityGroup, f64, &str, usize, &str) -> Entity,
    redacting_rules: RuleSet,
) {
    let text = "a@x.com then b@y.com";
    let first = text.find("a@x.com").expect("存在");
    let second = text.find("b@y.com").expect("存在");

    // 故意乱序传入，输出仍须按出现位置排列。
    let result = redact(
        text,
        &[
            entity(EntityGroup::PrivateEmail, 0.9, text, second, "b@y.com"),
            entity(EntityGroup::PrivateEmail, 0.9, text, first, "a@x.com"),
        ],
        &redacting_rules,
    );

    assert!(result.spans[0].byte_start < result.spans[1].byte_start);
    assert_eq!(
        result.redacted,
        "<redacted_private_email> then <redacted_private_email>"
    );
}

#[rstest]
fn entities_outside_the_text_are_ignored_rather_than_panicking(redacting_rules: RuleSet) {
    let text = "short";
    // 越界与未对齐的偏移不可能来自解码器，但上层数据可能被破坏；宁可跳过也不能 panic 或误替换。
    let bogus = Entity {
        entity_group: EntityGroup::Secret,
        score: 0.9,
        start: 2,
        end: 99,
        word: "ort".to_owned(),
    };

    let result = redact(text, &[bogus], &redacting_rules);
    assert_eq!(result.redacted, text);
    assert!(result.spans.is_empty());
}

#[rstest]
fn overlapping_spans_do_not_corrupt_the_result(
    entity: impl Fn(EntityGroup, f64, &str, usize, &str) -> Entity,
    redacting_rules: RuleSet,
) {
    let text = "abcdefghij";
    let result = redact(
        text,
        &[
            entity(EntityGroup::Secret, 0.9, text, 0, "abcdef"),
            entity(EntityGroup::Secret, 0.9, text, 4, "efghij"),
        ],
        &redacting_rules,
    );

    assert_eq!(result.redacted, "<redacted_secret>");
}

#[rstest]
fn the_placeholder_names_the_entity_group(
    entity: impl Fn(EntityGroup, f64, &str, usize, &str) -> Entity,
    redacting_rules: RuleSet,
) {
    for group in [
        EntityGroup::AccountNumber,
        EntityGroup::PrivateAddress,
        EntityGroup::PrivateDate,
        EntityGroup::PrivateEmail,
        EntityGroup::PrivatePerson,
        EntityGroup::PrivatePhone,
        EntityGroup::PrivateUrl,
        EntityGroup::Secret,
    ] {
        let text = "value";
        let result = redact(
            text,
            &[entity(group, 0.99, text, 0, "value")],
            &redacting_rules,
        );
        assert_eq!(result.redacted, format!("<redacted_{group}>"));
    }
}

#[rstest]
#[case(
    "smtp_password = \"mailpw123\"",
    "pw",
    "smtp_password = \"<redacted_secret>\""
)]
#[case(
    "db = \"postgres://user:s3cretpw@host/db\"",
    "cretpw",
    "db = \"<redacted_secret>\""
)]
fn partial_credentials_are_removed_completely(
    entity: impl Fn(EntityGroup, f64, &str, usize, &str) -> Entity,
    redacting_rules: RuleSet,
    #[case] text: &str,
    #[case] fragment: &str,
    #[case] expected: &str,
) {
    let result = redact(
        text,
        &[entity(
            EntityGroup::Secret,
            0.55,
            text,
            text.find(fragment).unwrap(),
            fragment,
        )],
        &redacting_rules,
    );
    assert_eq!(result.redacted, expected);
}

#[rstest]
fn embedded_image_is_preserved_while_adjacent_password_is_removed(
    entity: impl Fn(EntityGroup, f64, &str, usize, &str) -> Entity,
    redacting_rules: RuleSet,
) {
    let image = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aS1sAAAAASUVORK5CYII=";
    let text = format!("image=\"{image}\" password=\"mailpw123\"");
    let result = redact(
        &text,
        &[
            entity(
                EntityGroup::Secret,
                0.99,
                &text,
                text.find("CAQAA").unwrap(),
                "CAQAA",
            ),
            entity(
                EntityGroup::Secret,
                0.5,
                &text,
                text.find("pw123").unwrap(),
                "pw123",
            ),
        ],
        &redacting_rules,
    );
    assert_eq!(
        result.redacted,
        format!("image=\"{image}\" password=\"<redacted_secret>\"")
    );
}

#[rstest]
fn a_confirmed_credential_is_removed_wherever_it_appears_inside_a_word(
    entity: impl Fn(EntityGroup, f64, &str, usize, &str) -> Entity,
    redacting_rules: RuleSet,
) {
    let text = "password=\"mailpw123\" repeat=\"mailpw123\" other=\"xmailpw123x\"";
    let secret = text.find("pw123").expect("存在");
    let result = redact(
        text,
        &[entity(EntityGroup::Secret, 0.6, text, secret, "pw123")],
        &redacting_rules,
    );
    // 已确认的凭据出现在更长的词里，那个词本身就是凭据的一部分：整词抹去。
    // 只抹命中范围会在原文里留下 `xmail…x` 这样的碎片，碎片拼起来仍是那段凭据。
    assert_eq!(
        result.redacted,
        "password=\"<redacted_secret>\" repeat=\"<redacted_secret>\" other=\"<redacted_secret>\""
    );
}
