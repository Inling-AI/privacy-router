//! 确定性过滤器的识别契约：命中范围必须能直接切出原文，且只认形状与判据都成立的文本。

use privacy_filter::{Entity, EntityGroup, Filter, FilterKind, Filters};
use rstest::rstest;

/// 自定义过滤器：演示扩展点——新增一种识别方式只需实现这个 trait。
struct KeywordFilter {
    group: EntityGroup,
    needle: &'static str,
}

impl Filter for KeywordFilter {
    fn group(&self) -> EntityGroup {
        self.group
    }

    fn kind(&self) -> FilterKind {
        FilterKind::Additive
    }

    fn recognize(&self, text: &str) -> Vec<Entity> {
        text.match_indices(self.needle)
            .map(|(start, found)| Entity {
                entity_group: self.group,
                score: privacy_filter::PatternFilter::SCORE,
                start,
                end: start + found.len(),
                word: found.to_owned(),
            })
            .collect()
    }
}

#[rstest]
#[case("联系 alice@example.com 收", vec![(EntityGroup::PrivateEmail, "alice@example.com")])]
#[case("git@github.com:openai/privacy-filter.git", vec![(EntityGroup::PrivateEmail, "git@github.com")])]
#[case("a@b.co.后接句号", vec![(EntityGroup::PrivateEmail, "a@b.co")])]
#[case("-rwxr-xr-x@ 1 timmyovo wheel 15855616 Sep 12 00:31 /tmp/a.db", vec![])]
#[case("依赖升级到 pkg@1.2.3 之后", vec![])]
#[case("用户<alice@example.com>的地址", vec![(EntityGroup::PrivateEmail, "alice@example.com")])]
fn emails_are_recognized_by_shape(#[case] text: &str, #[case] expected: Vec<(EntityGroup, &str)>) {
    assert_hits(text, expected);
}

#[rstest]
#[case("联系电话 13812345678。", vec![(EntityGroup::PrivatePhone, "13812345678")])]
#[case("号码是 19912345678", vec![(EntityGroup::PrivatePhone, "19912345678")])]
#[case("订单号 13812345678A", vec![])]
#[case("手机号 12812345678", vec![])]
#[case("固定电话 010-12345678", vec![])]
fn china_mobile_numbers_are_recognized_with_their_scope(
    #[case] text: &str,
    #[case] expected: Vec<(EntityGroup, &str)>,
) {
    assert_hits(text, expected);
}

#[rstest]
#[case("身份证 110101199003077213 已登记", vec![(EntityGroup::IdCard, "110101199003077213")])]
#[case("身份证 110101199003077212 是改过的号", vec![])]
#[case("编号 91110000100000000M", vec![])]
fn id_cards_are_recognized_by_checksum(
    #[case] text: &str,
    #[case] expected: Vec<(EntityGroup, &str)>,
) {
    assert_hits(text, expected);
}

#[rstest]
#[case("卡号 6222021234567894 已解绑", vec![(EntityGroup::BankCard, "6222021234567894")])]
#[case("卡号 6222021234567895 校验不过", vec![])]
#[case("订单 1234567890123456", vec![])]
fn bank_cards_need_issuer_prefix_and_luhn(
    #[case] text: &str,
    #[case] expected: Vec<(EntityGroup, &str)>,
) {
    assert_hits(text, expected);
}

#[rstest]
#[case("统一社会信用代码 91350100M000100Y43", vec![(EntityGroup::CreditCode, "91350100M000100Y43")])]
#[case("统一社会信用代码 91350100M000100Y44", vec![])]
fn credit_codes_are_recognized_by_checksum(
    #[case] text: &str,
    #[case] expected: Vec<(EntityGroup, &str)>,
) {
    assert_hits(text, expected);
}

/// IP 只认公网地址。形状合法还不够：回环、私有、链路本地、运营商级 NAT 与保留段都是
/// 本地环境的坐标，报出来只会让每一次本机调试都变成一次告警。
#[rstest]
#[case("网关 93.184.216.34 不可达", vec![(EntityGroup::IpAddress, "93.184.216.34")])]
#[case("解析到 2606:4700:4700::1111", vec![(EntityGroup::IpAddress, "2606:4700:4700::1111")])]
#[case("本机 127.0.0.1 起服务", vec![])]
#[case("内网 10.1.2.3 打通", vec![])]
#[case("链路本地 169.254.10.10 不可用", vec![])]
#[case("保留段 240.0.0.1 无效", vec![])]
#[case("文档示例 203.0.113.9 不可达", vec![])]
#[case("运营商级 NAT 100.64.0.5 已分配", vec![])]
#[case("回环 ::1 是自指", vec![])]
#[case("链路本地 fe80::1 只在本地", vec![])]
#[case("版本 10.0.0.256 不存在", vec![])]
#[case("更长的点分数字 1.2.3.4.5 不是地址", vec![])]
fn only_public_addresses_are_recognized(
    #[case] text: &str,
    #[case] expected: Vec<(EntityGroup, &str)>,
) {
    assert_hits(text, expected);
}

#[rstest]
#[case("网卡 aa:bb:cc:dd:ee:ff 已上线", vec![(EntityGroup::MacAddress, "aa:bb:cc:dd:ee:ff")])]
#[case("网卡 aa-bb-cc-dd-ee-ff 已上线", vec![(EntityGroup::MacAddress, "aa-bb-cc-dd-ee-ff")])]
#[case("十六进制 aabbccddeeff 不是 MAC", vec![])]
fn mac_addresses_are_recognized_by_shape(
    #[case] text: &str,
    #[case] expected: Vec<(EntityGroup, &str)>,
) {
    assert_hits(text, expected);
}

/// MAC 的六组十六进制同时是合法 IPv6；只允许报一个类别，且必须是更具体的那个。
/// MAC 形状的文本也可能是合法 IPv6：`::` 开头的压缩写法正好把六组十六进制补成八段。
/// 同一段文本只允许报一个类别，且必须是证据更具体的那个。
#[rstest]
fn overlapping_categories_report_the_more_specific_one() {
    let hits = Filters::deterministic().recognize("网卡 ::aa:bb:cc:dd:ee:ff 已上线");

    assert_eq!(hits.len(), 1, "同一段文本只能有一条判定：{hits:?}");
    assert_eq!(hits[0].entity_group, EntityGroup::MacAddress);
}

#[rstest]
fn authoritative_categories_drop_the_probabilistic_hit() {
    let filters = Filters::deterministic();
    let from_model = vec![privacy_filter::Entity {
        entity_group: EntityGroup::PrivateEmail,
        score: 0.99,
        start: 0,
        end: 12,
        word: "a@b.co".to_owned(),
    }];

    let combined = filters.combine("a@b.co", from_model);

    assert_eq!(combined.len(), 1);
    assert_eq!(combined[0].score, privacy_filter::PatternFilter::SCORE);
}

#[rstest]
fn additive_categories_keep_the_probabilistic_hit_beside_the_deterministic_one() {
    let filters = Filters::deterministic();
    let from_model = vec![privacy_filter::Entity {
        entity_group: EntityGroup::PrivatePhone,
        score: 0.9,
        start: 29,
        end: 39,
        word: "+1 415 555".to_owned(),
    }];

    let combined = filters.combine("联系电话 13812345678 与 +1 415 555", from_model);

    assert_eq!(combined.len(), 2, "{combined:?}");
    assert!(
        combined
            .iter()
            .any(|hit| hit.score == privacy_filter::PatternFilter::SCORE)
    );
    assert!(combined.iter().any(|hit| hit.score == 0.9));
}

/// 自定义过滤器与内置过滤器共用同一条合并路径：新增识别方式不必改动别的过滤器。
#[rstest]
fn a_custom_filter_participates_in_the_same_merge() {
    let mut filters = Filters::deterministic();
    filters.push(KeywordFilter {
        group: EntityGroup::Secret,
        needle: "INTERNAL-",
    });

    let hits = filters.recognize("工单 INTERNAL- 已归档");

    assert_eq!(
        hits.iter()
            .map(|hit| (hit.entity_group, hit.word.as_str()))
            .collect::<Vec<_>>(),
        vec![(EntityGroup::Secret, "INTERNAL-")]
    );
}

fn assert_hits(text: &str, expected: Vec<(EntityGroup, &str)>) {
    let hits = Filters::deterministic().recognize(text);
    let actual: Vec<(EntityGroup, &str)> = hits
        .iter()
        .map(|hit| {
            // 偏移与原文必须自洽：上层替换不做子串搜索，错位就等于改错地方。
            assert_eq!(&text[hit.start..hit.end], hit.word);
            assert_eq!(hit.score, privacy_filter::PatternFilter::SCORE);
            (hit.entity_group, hit.word.as_str())
        })
        .collect();
    assert_eq!(actual, expected);
}
