-- 邮箱改为确定性识别后，预置规则不再筛长度。
--
-- 邮箱的形状现在由识别阶段的确定性模式给出（`EntityGroup::structured`），能产出邮箱实体
-- 的内容一定已经是完整地址，规则里再要求「字符数大于 4」只是把识别阶段的结论重算一遍。
-- 置信度条件保留：确定性命中的得分固定为 1.0，这条门槛不再筛掉真邮箱，只是维持
-- 「规则只认高置信度命中」这一不变式。
--
-- 只有逐字等于旧版预置的行才是升级对象。身份判定覆盖全部定义字段：来源、标识、名称、
-- 优先级、动作、条件。管理员改过其中任何一项，这条规则就属于他们的配置，升级不覆盖。
-- 开关（enabled）只是运行状态，扭转它不改变定义，因此不参与判定。
-- 预置规则的名称历来等于标识，`name = id` 即「名称未被改过」。

UPDATE rules
   SET condition_json = '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"private_email"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}}]}',
       updated_at = CAST(strftime('%s', 'now') AS INTEGER) * 1000
 WHERE source = 'builtin'
   AND action = 'redact'
   AND priority = 100
   AND name = id
   AND id = 'builtin.redact.private_email'
   AND condition_json = '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"private_email"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}},{"kind":"filter","filter":{"kind":"character_length","operator":"greater","value":4}}]}';
