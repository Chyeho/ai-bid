package com.ithsd.smart_tender.mapper;

import com.baomidou.mybatisplus.core.mapper.BaseMapper;
import com.ithsd.smart_tender.model.entity.AuditTask;
import org.apache.ibatis.annotations.Mapper;
import org.apache.ibatis.annotations.Select;
import org.apache.ibatis.annotations.Param;

import java.util.List;
import java.util.Map;

@Mapper
public interface AuditTaskMapper extends BaseMapper<AuditTask> {

    /**
     * 按天统计本周指定 bidId 的审核任务数量
     * @param bidIds 投标ID列表
     * @return 列表元素为 Map，每个 Map 包含 day_date(日期) 和 count(数量) 两个键
     */
    @Select("""
        <script>
        SELECT
            DATE_FORMAT(create_time, '%Y-%m-%d') AS day_date,
            COUNT(*) AS count
        FROM audit_task
        WHERE
            1 = 1
            <if test="bidIds != null and bidIds.size() > 0">
                AND bid_id IN
                <foreach item="item" collection="bidIds" open="(" separator="," close=")">
                    #{item}
                </foreach>
            </if>
            AND create_time IS NOT NULL
            AND YEARWEEK(DATE_FORMAT(create_time, '%Y-%m-%d'), 1) = YEARWEEK(CURDATE(), 1)
        GROUP BY day_date
        ORDER BY day_date
        </script>
        """)
    @org.apache.ibatis.annotations.ResultType(java.util.Map.class)
    List<Map<String, Object>> countByWeek(@Param("bidIds") List<Long> bidIds);
}
