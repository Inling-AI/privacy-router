-- 新增五个形状固定的类别：身份证、银行卡、统一社会信用代码、MAC、IP，各一条抹去规则。
--
-- 这些类别的命中由确定性过滤器给出，形状已经是一次形式校验，因此规则只保留置信度门槛。
-- 手机号不在此列：它由国内移动号段过滤器与模型并行识别，模型在该类别上仍然发言，长度门槛
-- 继续用来挡住概率命中的短噪声。
--
-- 新规则只在标识不存在时插入：预置只安装一次的约定不变，管理员若已经手工建过同名规则，
-- 这里既不改写也不覆盖。

WITH new_rules(id, condition_json) AS (
    VALUES
        ('builtin.redact.id_card', '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"id_card"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}}]}'),
        ('builtin.redact.bank_card', '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"bank_card"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}}]}'),
        ('builtin.redact.credit_code', '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"credit_code"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}}]}'),
        ('builtin.redact.mac_address', '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"mac_address"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}}]}'),
        ('builtin.redact.ip_address', '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"ip_address"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}}]}')
)
INSERT INTO rules (id, name, priority, condition_json, action, enabled, source, created_at, updated_at)
SELECT new_rules.id,
       new_rules.id,
       100,
       new_rules.condition_json,
       'redact',
       1,
       'builtin',
       CAST(strftime('%s', 'now') AS INTEGER) * 1000,
       CAST(strftime('%s', 'now') AS INTEGER) * 1000
  FROM new_rules
 WHERE NOT EXISTS (SELECT 1 FROM rules WHERE rules.id = new_rules.id);
