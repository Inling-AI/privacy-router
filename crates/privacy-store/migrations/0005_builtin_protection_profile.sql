-- 预置保护档位改版。每个受保护类别只留一条规则：命中「足够可信且足够长」就抹去。
--
--   * 删除全部「短匹配放行」反向规则：它们的条件恰好是拦截门槛的补集，在默认的放行兜底
--     之下从不生效，留在列表里只会让人以为拦截规则写反了。
--   * 日期不再设档位：预置规则不再认领日期，抹不抹由兜底策略和管理员规则决定。
--   * 姓名门槛从「字符数大于 1」提高到「字符数大于 4」。
--
-- 只有逐字等于旧版预置的行才是升级对象。身份判定覆盖全部定义字段：来源、标识、名称、
-- 优先级、动作、条件。管理员改过其中任何一项，这条规则就属于他们的配置，升级既不覆盖
-- 也不删除。开关（enabled）只是运行状态，扭转它不改变定义，因此不参与判定。
-- 预置规则的名称历来等于标识，`name = id` 即「名称未被改过」。

-- 旧版「短匹配放行」规则的原样定义，逐字冻结自迁移前的 builtin.rs。
WITH legacy_release(id, condition_json) AS (
    VALUES
        ('builtin.release.short.account_number', '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"account_number"}},{"kind":"filter","filter":{"kind":"character_length","operator":"less","value":4}}]}'),
        ('builtin.release.short.private_address', '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"private_address"}},{"kind":"filter","filter":{"kind":"character_length","operator":"less","value":5}}]}'),
        ('builtin.release.short.private_date', '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"private_date"}},{"kind":"filter","filter":{"kind":"character_length","operator":"less","value":4}}]}'),
        ('builtin.release.short.private_email', '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"private_email"}},{"kind":"filter","filter":{"kind":"character_length","operator":"less","value":5}}]}'),
        ('builtin.release.short.private_person', '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"private_person"}},{"kind":"filter","filter":{"kind":"character_length","operator":"less","value":2}}]}'),
        ('builtin.release.short.private_phone', '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"private_phone"}},{"kind":"filter","filter":{"kind":"character_length","operator":"less","value":7}}]}'),
        ('builtin.release.short.private_url', '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"private_url"}},{"kind":"filter","filter":{"kind":"character_length","operator":"less","value":5}}]}'),
        ('builtin.release.short.secret', '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"secret"}},{"kind":"filter","filter":{"kind":"character_length","operator":"less","value":8}}]}')
)
DELETE FROM rules
 WHERE source = 'builtin'
   AND action = 'release'
   AND priority = 400
   AND name = id
   AND EXISTS (
           SELECT 1
             FROM legacy_release
            WHERE legacy_release.id = rules.id
              AND legacy_release.condition_json = rules.condition_json
       );

-- 日期档位整体撤掉：条件 JSON 逐字等于旧版才删。
DELETE FROM rules
 WHERE source = 'builtin'
   AND action = 'redact'
   AND priority = 100
   AND name = id
   AND id = 'builtin.redact.private_date'
   AND condition_json = '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"private_date"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}},{"kind":"filter","filter":{"kind":"character_length","operator":"greater","value":3}}]}';

-- 姓名门槛抬高：仍等于旧版预置才改写条件。
UPDATE rules
   SET condition_json = '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"private_person"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}},{"kind":"filter","filter":{"kind":"character_length","operator":"greater","value":4}}]}',
       updated_at = CAST(strftime('%s', 'now') AS INTEGER) * 1000
 WHERE source = 'builtin'
   AND action = 'redact'
   AND priority = 100
   AND name = id
   AND id = 'builtin.redact.private_person'
   AND condition_json = '{"kind":"all","conditions":[{"kind":"filter","filter":{"kind":"entity","group":"private_person"}},{"kind":"filter","filter":{"kind":"confidence","operator":"greater","value":0.8}},{"kind":"filter","filter":{"kind":"character_length","operator":"greater","value":1}}]}';
