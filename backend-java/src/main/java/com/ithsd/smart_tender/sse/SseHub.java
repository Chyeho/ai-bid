package com.ithsd.smart_tender.sse;

import io.micrometer.core.instrument.Counter;
import io.micrometer.core.instrument.MeterRegistry;
import io.micrometer.core.instrument.simple.SimpleMeterRegistry;
import com.ithsd.smart_tender.model.enums.SseEventTypeEnum;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.stereotype.Component;
import org.springframework.util.StringUtils;
import org.springframework.web.servlet.mvc.method.annotation.SseEmitter;

import java.io.IOException;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

@Component
public class SseHub {
    private static final long SSE_TIMEOUT_MS = 30 * 60 * 1000L;
    private static final Logger log = LoggerFactory.getLogger(SseHub.class);
    private final ConcurrentHashMap<String, ConcurrentHashMap<String, SseEmitter>> emitterStore = new ConcurrentHashMap<>();
    private final Counter emitFailureCounter;
    private final Counter subscribeCounter;
    private final RedisSseConnectionStateStore redisStore;

    public SseHub(RedisSseConnectionStateStore redisStore) {
        this(null, redisStore);
    }

    @Autowired
    public SseHub(MeterRegistry meterRegistry, RedisSseConnectionStateStore redisStore) {
        MeterRegistry registry = meterRegistry == null ? new SimpleMeterRegistry() : meterRegistry;
        this.emitFailureCounter = registry.counter("audit.sse.emit.failure.total");
        this.subscribeCounter = registry.counter("audit.sse.subscribe.total");
        this.redisStore = redisStore;
        registry.gauge("audit.sse.connections.total", this, SseHub::totalConnections);
    }

    public SseEmitter subscribe(String taskId) {
        SseEmitter emitter = new SseEmitter(SSE_TIMEOUT_MS);
        String emitterId = UUID.randomUUID().toString();
        emitterStore.computeIfAbsent(taskId, key -> new ConcurrentHashMap<>()).put(emitterId, emitter);
        subscribeCounter.increment();
        
        // Sync to Redis
        int count = taskConnections(taskId);
        redisStore.upsert(taskId, count, "connected");
        
        log.info("sse subscribed, taskId={}, taskConnections={}, totalConnections={}", taskId, count, totalConnections());
        emitter.onCompletion(() -> remove(taskId, emitterId));
        emitter.onTimeout(() -> remove(taskId, emitterId));
        emitter.onError(ex -> remove(taskId, emitterId));
        return emitter;
    }

    public void emit(String taskId, SseEventTypeEnum eventType, Object data) {
        emit(taskId, eventType, data, null);
    }

    public void emit(String taskId, SseEventTypeEnum eventType, Object data, String eventId) {
        ConcurrentHashMap<String, SseEmitter> emitters = emitterStore.get(taskId);
        if (emitters == null || emitters.isEmpty()) {
            return;
        }
        for (Map.Entry<String, SseEmitter> entry : emitters.entrySet()) {
            try {
                entry.getValue().send(buildEvent(eventType, data, eventId));
            } catch (IOException | IllegalStateException ex) {
                emitFailureCounter.increment();
                log.warn("sse emit failed, taskId={}, event={}", taskId, eventType.getEventName(), ex);
                remove(taskId, entry.getKey());
            }
        }
        redisStore.refreshTtl(taskId);
    }

    public void emitToEmitter(SseEmitter emitter, SseEventTypeEnum eventType, Object data, String eventId) throws IOException {
        emitter.send(buildEvent(eventType, data, eventId));
    }

    public void close(String taskId) {
        ConcurrentHashMap<String, SseEmitter> emitters = emitterStore.remove(taskId);
        if (emitters == null) {
            return;
        }
        for (SseEmitter emitter : emitters.values()) {
            emitter.complete();
        }
        log.info("sse closed, taskId={}, totalConnections={}", taskId, totalConnections());
    }

    public int totalConnections() {
        return emitterStore.values().stream().mapToInt(Map::size).sum();
    }

    public int taskConnections(String taskId) {
        ConcurrentHashMap<String, SseEmitter> emitters = emitterStore.get(taskId);
        if (emitters == null) {
            return 0;
        }
        return emitters.size();
    }

    private void remove(String taskId, String emitterId) {
        ConcurrentHashMap<String, SseEmitter> emitters = emitterStore.get(taskId);
        if (emitters == null) {
            return;
        }
        emitters.remove(emitterId);
        if (emitters.isEmpty()) {
            emitterStore.remove(taskId);
        }
        int count = taskConnections(taskId);
        if (count > 0) {
            redisStore.upsert(taskId, count, "connected");
        } else {
            redisStore.delete(taskId);
        }
    }

    private SseEmitter.SseEventBuilder buildEvent(SseEventTypeEnum eventType, Object data, String eventId) {
        SseEmitter.SseEventBuilder builder = SseEmitter.event().name(eventType.getEventName()).data(data);
        if (StringUtils.hasText(eventId)) {
            builder.id(eventId);
        }
        return builder;
    }
}
