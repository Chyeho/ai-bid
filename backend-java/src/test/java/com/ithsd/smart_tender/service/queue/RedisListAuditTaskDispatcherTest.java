package com.ithsd.smart_tender.service.queue;

import com.ithsd.smart_tender.service.engine.queue.AuditQueueProperties;
import com.ithsd.smart_tender.service.engine.queue.RedisListAuditTaskDispatcher;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;
import org.springframework.data.redis.core.ListOperations;
import org.springframework.data.redis.core.StringRedisTemplate;

import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.*;

@ExtendWith(MockitoExtension.class)
class RedisListAuditTaskDispatcherTest {

    @Mock
    private StringRedisTemplate redisTemplate;

    @Mock
    private ListOperations<String, String> listOperations;

    @Mock
    private AuditQueueProperties queueProperties;

    @InjectMocks
    private RedisListAuditTaskDispatcher dispatcher;

    @Test
    void dispatch() {
        when(redisTemplate.opsForList()).thenReturn(listOperations);
        when(queueProperties.getStreamKey()).thenReturn("queue:audit:tasks");

        dispatcher.dispatch("task_123");

        verify(listOperations).rightPush(eq("queue:audit:tasks"), eq("task_123"));
    }
}
