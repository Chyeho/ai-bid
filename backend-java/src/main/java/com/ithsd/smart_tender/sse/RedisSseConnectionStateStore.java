package com.ithsd.smart_tender.sse;

import com.fasterxml.jackson.databind.ObjectMapper;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.data.redis.core.StringRedisTemplate;
import org.springframework.stereotype.Component;

import java.time.LocalDateTime;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.TimeUnit;

@Component
public class RedisSseConnectionStateStore {
    private static final Logger log = LoggerFactory.getLogger(RedisSseConnectionStateStore.class);
    private static final String KEY_PREFIX = "sse:connection:";
    private static final long TTL_MINUTES = 10;

    private final StringRedisTemplate redisTemplate;
    private final ObjectMapper objectMapper;

    public RedisSseConnectionStateStore(StringRedisTemplate redisTemplate, ObjectMapper objectMapper) {
        this.redisTemplate = redisTemplate;
        this.objectMapper = objectMapper;
    }

    public void upsert(String taskId, int connectionCount, String status) {
        try {
            Map<String, Object> state = new HashMap<>();
            state.put("taskId", taskId);
            state.put("connections", connectionCount);
            state.put("state", status);
            state.put("updatedAt", LocalDateTime.now().toString());

            String json = objectMapper.writeValueAsString(state);
            String key = KEY_PREFIX + taskId;
            redisTemplate.opsForValue().set(key, json, TTL_MINUTES, TimeUnit.MINUTES);
        } catch (Exception ex) {
            log.warn("failed to update sse connection state in redis, taskId={}", taskId, ex);
        }
    }

    public void delete(String taskId) {
        try {
            String key = KEY_PREFIX + taskId;
            redisTemplate.delete(key);
        } catch (Exception ex) {
            log.warn("failed to delete sse connection state from redis, taskId={}", taskId, ex);
        }
    }

    public void refreshTtl(String taskId) {
        try {
            String key = KEY_PREFIX + taskId;
            redisTemplate.expire(key, TTL_MINUTES, TimeUnit.MINUTES);
        } catch (Exception ex) {
            log.warn("failed to refresh sse connection ttl in redis, taskId={}", taskId, ex);
        }
    }
}
