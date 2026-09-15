//! 形状固定的类别：确定性过滤器与它们各自的判据。
//!
//! 邮箱是全球通行的形式标准：一个地址要么符合形状，要么就不存在，「像不像」没有任何余地。
//! 国内移动号码、身份证、银行卡、统一社会信用代码、MAC、IP 同样是形式规则；区别只在于各地
//! 号码格式不同，模式覆盖到什么范围必须写清楚——所以手机号过滤器是**追加**而非**取代**。
//!
//! 每个过滤器认什么、命中意味着什么，由 [`Filter::kind`] 声明；上层不再自己推断。

use crate::filter::{Filter, FilterKind};
use crate::{Entity, EntityGroup};
use regex::Regex;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::LazyLock;

/// 正则给出的候选命中，交回给过滤器自己裁决。
///
/// 正则只能表达字符形状；「两侧是不是更长词的一部分」「校验位对不对」由各自的过滤器决定，
/// 因此这里只提供原文位置与邻接字符。
pub struct Candidate<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

impl<'a> Candidate<'a> {
    /// 命中文本。
    pub fn word(&self) -> &'a str {
        &self.text[self.start..self.end]
    }

    /// 紧邻命中前后的字符；它们决定命中是不是被更长的词包住。
    fn surroundings(&self) -> (Option<char>, Option<char>) {
        (
            self.text[..self.start].chars().next_back(),
            self.text[self.end..].chars().next(),
        )
    }

    /// 两侧都不是字母或数字：`13800138000` 要，`a13800138000b` 里夹的那一段不要。
    fn stands_alone(&self) -> bool {
        let (before, after) = self.surroundings();
        !before.is_some_and(|character| character.is_ascii_alphanumeric())
            && !after.is_some_and(|character| character.is_ascii_alphanumeric())
    }

    /// 两侧都不是数字或点：`1.2.3.4` 要，`1.2.3.4.5` 里切出来的一段不要。
    fn stands_alone_as_dotted_number(&self) -> bool {
        let (before, after) = self.surroundings();
        !before.is_some_and(|character| character.is_ascii_digit() || character == '.')
            && !after.is_some_and(|character| character.is_ascii_digit() || character == '.')
    }

    /// GB 11643-1999：前 17 位加权求和，模 11 取校验字符。
    fn id_card_checksum(word: &str) -> bool {
        const WEIGHTS: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
        const CHECK: [u8; 11] = *b"10X98765432";
        let bytes = word.as_bytes();
        if bytes.len() != 18 {
            return false;
        }
        let mut sum = 0;
        for (index, weight) in WEIGHTS.iter().enumerate() {
            let Some(digit) = (bytes[index] as char).to_digit(10) else {
                return false;
            };
            sum += digit * weight;
        }
        bytes[17].to_ascii_uppercase() == CHECK[(sum % 11) as usize]
    }

    /// 统一社会信用代码：前 17 位按 31 进制加权求和，模 31 取校验字符。
    fn credit_code_checksum(word: &str) -> bool {
        const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKLMNPQRTUWXY";
        const WEIGHTS: [u32; 17] = [
            1, 3, 9, 27, 19, 26, 16, 17, 20, 29, 25, 13, 8, 24, 10, 30, 28,
        ];
        let bytes = word.as_bytes();
        if bytes.len() != 18 {
            return false;
        }
        let mut sum = 0;
        for (index, weight) in WEIGHTS.iter().enumerate() {
            let Some(value) = ALPHABET
                .iter()
                .position(|candidate| *candidate == bytes[index].to_ascii_uppercase())
            else {
                return false;
            };
            sum += value as u32 * weight;
        }
        let check = (31 - sum % 31) % 31;
        bytes[17].to_ascii_uppercase() == ALPHABET[check as usize]
    }

    /// 发卡行前缀：只有这些前缀的号码才当作卡号，防止把长数字串读成卡号。
    fn issuer_prefix_matches(word: &str) -> bool {
        const ISSUERS: [&str; 14] = [
            "4", "51", "52", "53", "54", "55", "34", "37", "35", "62", "60", "81", "88", "65",
        ];
        ISSUERS.iter().any(|prefix| word.starts_with(prefix))
    }

    /// Luhn 校验：自右向左隔位加倍，求和能被 10 整除。
    fn luhn_holds(word: &str) -> bool {
        let mut sum = 0;
        for (index, character) in word.chars().rev().enumerate() {
            let Some(digit) = character.to_digit(10) else {
                return false;
            };
            let value = if index % 2 == 1 { digit * 2 } else { digit };
            sum += if value > 9 { value - 9 } else { value };
        }
        sum % 10 == 0
    }

    /// 这段文本是不是一个**公网**地址字面量。
    ///
    /// 形状交给 [`IpAddr`] 的解析器裁决：前导零、越界段、不完整的 IPv6 都不是地址，MAC 那
    /// 六组十六进制本身也凑不出合法 IPv6。IPv4 另外要求不被更长的点分数字包住（`1.2.3.4.5` 里
    /// 切出来的一段不算）。
    ///
    /// 私有、回环、链路本地、组播与保留段不是「泄露的隐私」，而是本地环境的坐标：`127.0.0.1`
    /// 出现在哪都是本机自指，把它报出来只会让每一次本机调试都变成一次告警。因此这些地址根本
    /// 不算命中，连放行规则都不需要。
    fn is_public_address(&self) -> bool {
        match self.word().parse::<IpAddr>() {
            Ok(IpAddr::V4(address)) => {
                self.stands_alone_as_dotted_number() && Self::ipv4_is_public(address)
            }
            Ok(IpAddr::V6(address)) => self.stands_alone() && Self::ipv6_is_public(address),
            Err(_) => false,
        }
    }

    /// IANA 特殊用途注册表之外的 IPv4 才是公网地址。
    fn ipv4_is_public(address: Ipv4Addr) -> bool {
        let octets = address.octets();
        !(octets[0] == 0                                     // 0.0.0.0/8 本网络
            || address.is_private()                          // 10/8、172.16/12、192.168/16
            || (octets[0] == 100 && (64..128).contains(&octets[1])) // 100.64/10 运营商级 NAT
            || address.is_loopback()                         // 127/8
            || address.is_link_local()                       // 169.254/16
            || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0) // 192.0.0/24 IETF 协议分配
            || address.is_documentation()                    // 192.0.2/24、198.51.100/24、203.0.113/24
            || (octets[0] == 192 && octets[1] == 88 && octets[2] == 99) // 192.88.99/24 6to4 中继
            || (octets[0] == 198 && (octets[1] == 18 || octets[1] == 19)) // 198.18/15 基准测试
            || address.is_multicast()                        // 224/4
            || octets[0] >= 240) // 240/4 保留（含广播地址）
    }

    /// IANA 特殊用途注册表之外的 IPv6 才是公网地址；内嵌 IPv4 的写法随内嵌地址判定。
    fn ipv6_is_public(address: Ipv6Addr) -> bool {
        // `::ffff:0:0/96`（IPv4 映射）、`::/96`（IPv4 兼容）、`64:ff9b::/96`（NAT64）的可达性
        // 由内嵌的 IPv4 决定；`::` 与 `::1` 也走这条路，落回 0.0.0.0/8。
        if let Some(embedded) = address.to_ipv4() {
            return Self::ipv4_is_public(embedded);
        }
        let [first, second, third, fourth, ..] = address.segments();
        !(address.is_multicast()                                  // ff00::/8
            || address.is_unique_local()                          // fc00::/7
            || address.is_unicast_link_local()                    // fe80::/10
            || (first == 0x0100 && second == 0 && third == 0 && fourth == 0) // 100::/64 丢弃前缀
            || (first == 0x0064 && second == 0xff9b && third == 0x0001) // 64:ff9b:1::/48 本地 NAT64
            || (first == 0x2001 && second < 0x0200)               // 2001::/23 IETF 协议分配
            || (first == 0x2001 && second == 0x0db8)              // 2001:db8::/32 文档
            || first == 0x2002                                    // 2002::/16 6to4
            || (first == 0x3fff && second < 0x1000)               // 3fff::/20 文档
            || first == 0x5f00) // 5f00::/16 SRv6 SID
    }
}

/// 由正则与逐类别判据构成的确定性过滤器。
#[derive(Debug)]
pub struct PatternFilter {
    group: EntityGroup,
    kind: FilterKind,
    pattern: Regex,
}

impl PatternFilter {
    /// 确定性命中的得分。命中是事实，不是概率估计，也不代表任何不确定性。
    pub const SCORE: f64 = 1.0;
}

impl Filter for PatternFilter {
    fn group(&self) -> EntityGroup {
        self.group
    }

    fn kind(&self) -> FilterKind {
        self.kind
    }

    /// 找出文本中全部命中，按出现顺序返回且互不重叠。
    ///
    /// 偏移由正则引擎在字节上给出，因此天然落在 UTF-8 字符边界上。
    fn recognize(&self, text: &str) -> Vec<Entity> {
        self.pattern
            .find_iter(text)
            .filter(|found| {
                accepts(
                    self.group,
                    &Candidate {
                        text,
                        start: found.start(),
                        end: found.end(),
                    },
                )
            })
            .map(|found| Entity {
                entity_group: self.group,
                score: Self::SCORE,
                start: found.start(),
                end: found.end(),
                word: found.as_str().to_owned(),
            })
            .collect()
    }
}

/// 邮箱：本地部分 + `@` + 至少一个点分域名。
///
/// 末段必须是字母，因此 `pkg@1.2.3` 这类版本号不会被误判；`@` 之后必须紧跟域名，
/// 因此 `ls -l` 输出里的 `-rwxr-xr-x@` 也不会被误判。
static EMAIL: LazyLock<PatternFilter> = LazyLock::new(|| PatternFilter {
    group: EntityGroup::PrivateEmail,
    kind: FilterKind::Authoritative,
    pattern: Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9\-]+(?:\.[A-Za-z0-9\-]+)*\.[A-Za-z]{2,}")
        .expect("内置邮箱模式可用"),
});

/// 国内移动号码：11 位，`1` 开头，第二位 3-9。不含固话、400/800、境外号码。
static CHINA_MOBILE: LazyLock<PatternFilter> = LazyLock::new(|| PatternFilter {
    group: EntityGroup::PrivatePhone,
    kind: FilterKind::Additive,
    pattern: Regex::new(r"1[3-9][0-9]{9}").expect("内置手机号模式可用"),
});

/// 身份证：省份与出生日期由正则约束，校验位由判据裁决。
static ID_CARD: LazyLock<PatternFilter> = LazyLock::new(|| {
    PatternFilter {
    group: EntityGroup::IdCard,
    kind: FilterKind::Authoritative,
    pattern: Regex::new(
        r"[1-9][0-9]{5}(?:18|19|20)[0-9]{2}(?:0[1-9]|1[0-2])(?:0[1-9]|[12][0-9]|3[01])[0-9]{3}[0-9Xx]",
    )
    .expect("内置身份证模式可用"),
}
});

/// 统一社会信用代码：18 位，字符集不含容易混淆的 I、O、S、V、Z。
static CREDIT_CODE: LazyLock<PatternFilter> = LazyLock::new(|| PatternFilter {
    group: EntityGroup::CreditCode,
    kind: FilterKind::Authoritative,
    pattern: Regex::new(r"[0-9A-HJ-NPQRTUWXY]{2}[0-9]{6}[0-9A-HJ-NPQRTUWXY]{10}")
        .expect("内置信用代码模式可用"),
});

/// 银行卡：长度宽松，发卡行前缀与 Luhn 校验收紧。
static BANK_CARD: LazyLock<PatternFilter> = LazyLock::new(|| PatternFilter {
    group: EntityGroup::BankCard,
    kind: FilterKind::Authoritative,
    pattern: Regex::new(r"[0-9]{15,19}").expect("内置银行卡模式可用"),
});

/// MAC 地址：冒号或短横线分隔的六组两位十六进制。
static MAC_ADDRESS: LazyLock<PatternFilter> = LazyLock::new(|| PatternFilter {
    group: EntityGroup::MacAddress,
    kind: FilterKind::Authoritative,
    pattern: Regex::new(r"[0-9A-Fa-f]{2}(?:[:-][0-9A-Fa-f]{2}){5}").expect("内置 MAC 模式可用"),
});

/// IP 地址：IPv4 与 IPv6 共用一个过滤器，形状与可达性都由判据裁决。
static IP_ADDRESS: LazyLock<PatternFilter> = LazyLock::new(|| {
    PatternFilter {
    group: EntityGroup::IpAddress,
    kind: FilterKind::Authoritative,
    pattern: Regex::new(
        r"[0-9]{1,3}(?:\.[0-9]{1,3}){3}|(?:[0-9A-Fa-f]{1,4}:){2,7}(?::[0-9A-Fa-f]{1,4}){0,7}|[0-9A-Fa-f]{1,4}:(?::[0-9A-Fa-f]{1,4}){1,7}|::(?:[0-9A-Fa-f]{1,4}:){0,6}[0-9A-Fa-f]{1,4}",
    )
    .expect("内置 IP 模式可用"),
}
});

/// 内置确定性过滤器；新增一种识别方式就往这里加一个实例。
pub(crate) fn builtin() -> [&'static PatternFilter; 7] {
    [
        &EMAIL,
        &CHINA_MOBILE,
        &ID_CARD,
        &CREDIT_CODE,
        &BANK_CARD,
        &MAC_ADDRESS,
        &IP_ADDRESS,
    ]
}

/// 形状之外，这个类别的候选命中还要满足什么。
///
/// 逐个列出全部类别，新增类别时编译器会强制在这里做出选择。
fn accepts(group: EntityGroup, candidate: &Candidate<'_>) -> bool {
    match group {
        // 形状已经由正则表达；`@` 之后必须紧跟域名，没有额外判据。
        EntityGroup::PrivateEmail => true,
        EntityGroup::PrivatePhone => candidate.stands_alone() && candidate.word().len() == 11,
        EntityGroup::IdCard => {
            candidate.stands_alone() && Candidate::id_card_checksum(candidate.word())
        }
        EntityGroup::CreditCode => {
            candidate.stands_alone() && Candidate::credit_code_checksum(candidate.word())
        }
        // 只用 Luhn 不够：随机 16 位数字有十分之一会过，前缀是第二道筛子。
        EntityGroup::BankCard => {
            candidate.stands_alone()
                && Candidate::issuer_prefix_matches(candidate.word())
                && Candidate::luhn_holds(candidate.word())
        }
        EntityGroup::MacAddress => candidate.stands_alone(),
        EntityGroup::IpAddress => candidate.is_public_address(),
        // 没有确定性过滤器的类别不会有候选命中。
        EntityGroup::AccountNumber
        | EntityGroup::PrivateAddress
        | EntityGroup::PrivateDate
        | EntityGroup::PrivatePerson
        | EntityGroup::PrivateUrl
        | EntityGroup::Secret => false,
    }
}

impl EntityGroup {
    /// 重叠时谁说了算：数越小越优先，因为它的证据越具体。
    ///
    /// 逐个列出全部类别，新增类别时编译器会强制在这里做出选择。
    pub(crate) fn specificity(self) -> u8 {
        match self {
            Self::IdCard => 0,
            Self::CreditCode => 1,
            Self::BankCard => 2,
            Self::MacAddress => 3,
            Self::IpAddress => 4,
            Self::PrivatePhone => 5,
            Self::PrivateEmail => 6,
            Self::AccountNumber
            | Self::PrivateAddress
            | Self::PrivateDate
            | Self::PrivatePerson
            | Self::PrivateUrl
            | Self::Secret => 7,
        }
    }
}
