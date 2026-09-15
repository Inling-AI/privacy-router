-- 确定性类别的预置规则去掉置信度条件。
--
-- 邮箱、身份证、银行卡、统一社会信用代码、MAC、IP 的命中来自确定性过滤器：形状就是类别的
-- 定义，过滤器把得分写成常数 1.0。规则里的「置信度大于 0.8」因此是一个恒真的重算——它既
-- 不能筛掉任何命中，也不代表任何判断。这些类别只需要问「是不是这个类别」。
--
-- 手机号不在其中：它是追加型过滤器，模式只覆盖国内移动号段，模型在同一类别上仍然发言，
-- 置信度与长度门槛继续用来挡概率命中的噪声。
--
-- 只有逐字等于上一版预置的行才是升级对象。身份判定覆盖全部定义字段：来源、标识、名称、
-- 优先级、动作、条件；管理员改过其中任何一项，这条规则就属于他们的配置，升级不覆盖。
-- 开关（enabled）只是运行状态，扭转它不改变定义，因此不参与判定。

WITH replacements(id, previous_json, replacement_json) AS (
    VALUES
        ('builtin.redact.bank_card',
         '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"bank_card"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}}]}',
         '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"bank_card"}}]}'),
        ('builtin.redact.credit_code',
         '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"credit_code"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}}]}',
         '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"credit_code"}}]}'),
        ('builtin.redact.id_card',
         '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"id_card"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}}]}',
         '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"id_card"}}]}'),
        ('builtin.redact.ip_address',
         '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"ip_address"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}}]}',
         '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"ip_address"}}]}'),
        ('builtin.redact.mac_address',
         '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"mac_address"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}}]}',
         '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"mac_address"}}]}'),
        ('builtin.redact.private_email',
         '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"private_email"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}}]}',
         '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"private_email"}}]}')
)
UPDATE rules
   SET condition_json = (SELECT replacement_json FROM replacements WHERE replacements.id = rules.id),
       updated_at = CAST(strftime('%s', 'now') AS INTEGER) * 1000
 WHERE id IN (SELECT id FROM replacements)
   AND source = 'builtin'
   AND action = 'redact'
   AND priority = 100
   AND name = id
   AND condition_json = (SELECT previous_json FROM replacements WHERE replacements.id = rules.id);
