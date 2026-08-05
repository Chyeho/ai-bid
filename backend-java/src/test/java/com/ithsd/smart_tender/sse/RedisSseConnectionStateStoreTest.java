package com.ithsd.smart_tender.sse;

import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;
import org.springframework.data.redis.core.StringRedisTemplate;
import org.springframework.data.redis.core.ValueOperations;

import java.util.concurrent.TimeUnit;

import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.*;

@ExtendWith(MockitoExtension.class)
class RedisSseConnectionStateStoreTest {

    @Mock
    private StringRedisTemplate redisTemplate;

    @Mock
    private ValueOperations<String, String> valueOperations;

    @Mock
    private ObjectMapper objectMapper;

    @InjectMocks
    private RedisSseConnectionStateStore store;

    @Test
    void upsert() throws Exception {
        when(redisTemplate.opsForValue()).thenReturn(valueOperations);
        when(objectMapper.writeValueAsString(any())).thenReturn("{}");

        store.upsert("task_123", 1, "connected");

        verify(valueOperations).set(eq("sse:connection:task_123"), anyString(), eq(10L), eq(TimeUnit.MINUTES));
    }

    @Test
    void delete() {
        store.delete("task_123");
        verify(redisTemplate).delete("sse:connection:task_123");
    }

    @Test
    void refreshTtl() {
        store.refreshTtl("task_123");
        verify(redisTemplate).expire("sse:connection:task_123", 10L, TimeUnit.MINUTES);
    }
}
