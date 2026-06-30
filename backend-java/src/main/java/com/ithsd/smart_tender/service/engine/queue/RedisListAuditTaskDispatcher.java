package com.ithsd.smart_tender.service.engine.queue;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;
import org.springframework.data.redis.core.StringRedisTemplate;
import org.springframework.stereotype.Component;

@Component
@ConditionalOnProperty(name = "audit.queue.mode", havingValue = "redis-list")
public class RedisListAuditTaskDispatcher implements AuditTaskDispatcher {
    private static final Logger log = LoggerFactory.getLogger(RedisListAuditTaskDispatcher.class);
    private final StringRedisTemplate redisTemplate;
    private final AuditQueueProperties queueProperties;

    public RedisListAuditTaskDispatcher(StringRedisTemplate redisTemplate, AuditQueueProperties queueProperties) {
        this.redisTemplate = redisTemplate;
        this.queueProperties = queueProperties;
    }

    @Override
    public void dispatch(String taskId) {
        try {
            String key = queueProperties.getStreamKey(); // reusing streamKey config as list key
            redisTemplate.opsForList().rightPush(key, taskId);
            log.info("audit task dispatched to redis list, taskId={}, key={}", taskId, key);
        } catch (Exception ex) {
            log.error("failed to dispatch audit task to redis list, taskId={}", taskId, ex);
            throw ex;
        }
    }
}
