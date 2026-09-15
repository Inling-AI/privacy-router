-- 登记补上「这段原文是什么类别」。
--
-- 登记的身份是原文本身，类别不参与匹配：匹配永远是「这段原文相等」。类别只回答两个问题
-- 抹去后的占位词叫什么、内容池里的标签怎么写——因此它是登记的一部分，不是条件的重算。
--
-- 历史登记没有这一列，从判定记录里回填：同一段原文被判成过什么类别，判定记录里已经写着；
-- 取出现最多的那个，次数相同时按类别名定序，保证回填结果与查询顺序无关。

ALTER TABLE rules ADD COLUMN category TEXT;

UPDATE rules
   SET category = (
           SELECT spans.entity_group
             FROM spans
            WHERE spans.original_text = json_extract(rules.condition_json, '$.filter.pattern.text')
            GROUP BY spans.entity_group
            ORDER BY COUNT(*) DESC, spans.entity_group
            LIMIT 1
       )
 WHERE source = 'approval';

-- 判定记录已被清理的登记回填不到类别。类别不参与匹配，只决定占位词与列表标签；但装配
-- 规则集合时缺少它会让整个代理起不来。给它一个显式取值：宁可标签不准，也不能让保护缺席。
UPDATE rules SET category = 'secret' WHERE source = 'approval' AND category IS NULL;
