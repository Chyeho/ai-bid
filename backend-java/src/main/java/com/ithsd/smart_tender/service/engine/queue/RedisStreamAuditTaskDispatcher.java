package com.ithsd.smart_tender.service.engine.queue;

import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;
import org.springframework.data.redis.connection.stream.MapRecord;
import org.springframework.data.redis.core.StringRedisTemplate;
import org.springframework.stereotype.Component;

import java.util.Map;

@Component
@ConditionalOnProperty(prefix = "audit.queue", name = "mode", havingValue = "redis-stream")
public class RedisStreamAuditTaskDispatcher implements AuditTaskDispatcher {
    private final StringRedisTemplate redisTemplate;
    private final AuditQueueProperties queueProperties;

    public RedisStreamAuditTaskDispatcher(StringRedisTemplate redisTemplate, AuditQueueProperties queueProperties) {
        this.redisTemplate = redisTemplate;
        this.queueProperties = queueProperties;
    }

    @Override
    public void dispatch(String taskId) {
        MapRecord<String, String, String> record = MapRecord.create(queueProperties.getStreamKey(), Map.of(
                "taskId", taskId,
                "retry", "0"
        ));
        redisTemplate.opsForStream().add(record);
    }
}
