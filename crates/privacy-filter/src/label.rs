use serde::{Deserialize, Deserializer, Serialize, Serializer};
use strum::{AsRefStr, Display, EnumCount, EnumIter, EnumString, IntoEnumIterator};

/// 我们要报告的全部实体类别。这是上层（规则、审计、控制台）唯一的类别词表。
///
/// 其中一部分由模型判断，另一部分形状固定、由确定性过滤器给出——过滤器名单见
/// [`Filter`](crate::Filter)。这里不保存与 checkpoint 有关的任何顺序约定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, EnumCount, EnumString, AsRefStr, Display)]
#[strum(serialize_all = "snake_case")]
pub enum EntityGroup {
    AccountNumber,
    BankCard,
    CreditCode,
    IdCard,
    IpAddress,
    MacAddress,
    PrivateAddress,
    PrivateDate,
    PrivateEmail,
    PrivatePerson,
    PrivatePhone,
    PrivateUrl,
    Secret,
}

impl EntityGroup {
    /// 遍历全部类别；消费者不再各自维护一份类别数组。
    pub fn all() -> impl ExactSizeIterator<Item = Self> {
        Self::iter()
    }
}

/// 模型能给出的标签集合；声明顺序对应 checkpoint 中的类别顺序，`id` 由它算出。
///
/// 这是关于模型的事实：它只认得出这八类，且顺序是权重的一部分。我们要报告的类别可以比它多，
/// 两者的对应关系由 [`ModelLabel::category`] 独家给出。
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, EnumCount, EnumString, AsRefStr, Display)]
#[strum(serialize_all = "snake_case")]
#[repr(usize)]
pub enum ModelLabel {
    AccountNumber,
    PrivateAddress,
    PrivateDate,
    PrivateEmail,
    PrivatePerson,
    PrivatePhone,
    PrivateUrl,
    Secret,
}

impl ModelLabel {
    /// 这个模型标签对应哪个报告类别。
    ///
    /// 逐个列出全部标签，新增模型标签时编译器会强制在这里做出选择。
    pub fn category(self) -> EntityGroup {
        match self {
            Self::AccountNumber => EntityGroup::AccountNumber,
            Self::PrivateAddress => EntityGroup::PrivateAddress,
            Self::PrivateDate => EntityGroup::PrivateDate,
            Self::PrivateEmail => EntityGroup::PrivateEmail,
            Self::PrivatePerson => EntityGroup::PrivatePerson,
            Self::PrivatePhone => EntityGroup::PrivatePhone,
            Self::PrivateUrl => EntityGroup::PrivateUrl,
            Self::Secret => EntityGroup::Secret,
        }
    }
}

impl Serialize for EntityGroup {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_ref())
    }
}

impl<'de> Deserialize<'de> for EntityGroup {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// 实体内部的 BIOES 边界；背景由 [`TokenLabel::Outside`] 表达。
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, EnumCount, AsRefStr)]
#[repr(usize)]
pub enum Boundary {
    #[strum(serialize = "B")]
    Begin,
    #[strum(serialize = "I")]
    Inside,
    #[strum(serialize = "E")]
    End,
    #[strum(serialize = "S")]
    Single,
}

/// 类型化的 token 标签；编号、标签总数和状态转移都由此类型约束。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenLabel {
    Outside,
    Entity {
        group: ModelLabel,
        boundary: Boundary,
    },
}

impl TokenLabel {
    pub const COUNT: usize = 1 + ModelLabel::COUNT * Boundary::COUNT;

    pub fn id(self) -> usize {
        match self {
            Self::Outside => 0,
            Self::Entity { group, boundary } => {
                1 + group as usize * Boundary::COUNT + boundary as usize
            }
        }
    }

    pub fn from_id(id: usize) -> Option<Self> {
        if id == 0 {
            return Some(Self::Outside);
        }
        Some(Self::Entity {
            group: ModelLabel::iter().nth((id - 1) / Boundary::COUNT)?,
            boundary: Boundary::iter().nth((id - 1) % Boundary::COUNT)?,
        })
    }

    pub(crate) fn all() -> impl Iterator<Item = Self> {
        std::iter::once(Self::Outside).chain(ModelLabel::iter().flat_map(|group| {
            Boundary::iter().map(move |boundary| Self::Entity { group, boundary })
        }))
    }

    pub(crate) fn can_start(self) -> bool {
        matches!(
            self,
            Self::Outside
                | Self::Entity {
                    boundary: Boundary::Begin | Boundary::Single,
                    ..
                }
        )
    }

    pub(crate) fn can_end(self) -> bool {
        matches!(
            self,
            Self::Outside
                | Self::Entity {
                    boundary: Boundary::End | Boundary::Single,
                    ..
                }
        )
    }

    pub(crate) fn allows_next(self, next: Self) -> bool {
        if self.can_end() {
            return next.can_start();
        }
        matches!((self, next), (Self::Entity { group: a, .. }, Self::Entity {
            group: b, boundary: Boundary::Inside | Boundary::End,
        }) if a == b)
    }
}

impl std::fmt::Display for TokenLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Outside => f.write_str("O"),
            Self::Entity { group, boundary } => write!(f, "{}-{group}", boundary.as_ref()),
        }
    }
}
